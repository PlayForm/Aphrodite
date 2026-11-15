//! Build one context-engine archive with no session mutation.

use crate::{Session::Engine::Plan::Struct::Struct as Plan, State::Struct::AphroditeState};

/// Pack the middle turns into one retrievable payload and render its marker.
///
/// No session access beyond reading: the packed payload, the hash, the preview,
/// the marker and the prompt are all functions of the index and the templates, so
/// the whole plan can be built before anything is stored. `None` means the middle
/// turns could not be packed, and the caller then refuses rather than archiving
/// half a history.
///
/// The packed entries are the REAL recorded turns (turn → hash/summary/size),
/// never fabricated message content: the dylib never sees the message list.
pub(crate) fn Fn(State:&AphroditeState, Total:usize, HeadN:usize, TailN:usize, MiddleN:usize) -> Option<Plan> {
	// Sorted turns, oldest first (same ordering as `GetConvIndex`).
	let mut Turns:Vec<usize> = State.ConvIndex.keys().copied().collect();
	Turns.sort_unstable();

	let End = Total.saturating_sub(TailN);
	let MiddleKeys:Vec<usize> = Turns[HeadN..End].to_vec();

	let PackedEntries:Vec<serde_json::Value> = MiddleKeys
		.iter()
		.filter_map(|Turn| {
			State.ConvIndex.get(Turn).map(
				|(Hash, Summary, Size)| serde_json::json!({ "turn": Turn, "hash": Hash, "summary": Summary, "size": Size }),
			)
		})
		.collect();

	let Packed = match serde_json::to_string(&PackedEntries) {
		Ok(Packed) => Packed,
		Err(Error) => {
			tracing::warn!(error = %Error, "engine_maybe_archive: packing middle turns failed; skipping");

			return None;
		},
	};

	let Hash = headroom_core::ccr::compute_key(Packed.as_bytes());
	let Preview = Packed.chars().take(120).collect::<String>();
	let Marker = crate::Marker::Format::Entry::CCRMarker(&Hash, "context", Packed.len(), &Preview, None, None, None);
	let Rendered = crate::Flow::Prompt::Render::Fn::RenderPrompt(
		State,
		"engine_offload",
		&[("hash", &Hash), ("tail", &TailN.to_string())],
	);
	let Prompt = format!("{Marker}\n{Rendered}");

	Some(Plan { HeadN, TailN, MiddleN, MiddleKeys, Packed, Hash, Preview, Prompt })
}
