//! Build one context-engine archive with no session mutation.

use crate::{
	Session::Engine::{Gate::Enum::Enum as Gate, Plan::Struct::Struct as Plan},
	State::Struct::AphroditeState,
};

/// Pack the middle turns into one retrievable payload and render its marker.
///
/// No session access beyond reading: the packed payload, the hash, the preview,
/// the marker and the prompt are all functions of the index and the templates, so
/// the whole plan can be built before anything is stored. `Err` names WHY nothing
/// was built, so the caller reports the refusal it actually met instead of a
/// generic "unpackable".
///
/// The slice is taken through `get` rather than by indexing: the protect bounds come
/// from configuration, and a bound pair that leaves no middle (`HeadN > End`) is a
/// refusal, not a panic inside a hook.
///
/// The packed entries are the REAL recorded turns (turn → hash/summary/size),
/// never fabricated message content: the dylib never sees the message list.
pub(crate) fn Fn(State:&AphroditeState, Total:usize, HeadN:usize, TailN:usize, MiddleN:usize) -> Result<Plan, Gate> {
	// Sorted turns, oldest first (same ordering as `Index`).
	let mut Turns:Vec<usize> = State.ConvIndex.keys().copied().collect();
	Turns.sort_unstable();

	let End = Total.saturating_sub(TailN);
	let MiddleKeys:Vec<usize> = Turns.get(HeadN..End).ok_or(Gate::OutOfRange)?.to_vec();

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

			return Err(Gate::Unpackable);
		},
	};

	let Hash = headroom_core::ccr::compute_key(Packed.as_bytes());
	let Preview = Packed.chars().take(120).collect::<String>();
	let Marker = crate::Marker::Format::Entry::Fn(&Hash, "context", Packed.len(), &Preview, None, None, None);
	let Rendered =
		crate::Flow::Prompt::Render::Fn::Fn(State, "engine_offload", &[("hash", &Hash), ("tail", &TailN.to_string())]);
	let Prompt = format!("{Marker}\n{Rendered}");

	Ok(Plan { HeadN, TailN, MiddleN, MiddleKeys, Packed, Hash, Preview, Prompt })
}
