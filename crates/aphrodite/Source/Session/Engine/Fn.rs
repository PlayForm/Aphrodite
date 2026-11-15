//! The context-engine archiving entry.
//!
//! Decides from the conversation index alone whether to compact the middle of
//! the history: the dylib never receives the raw message list (Hermes holds
//! it) and has no token counts, so the index's own cap is the only "full"
//! reference available. Every gate runs before any mutation, so a refusal is a
//! true no-op - byte-identical behaviour to an engine that does not exist.

use crate::{
	Session::{Archive::Fn::ArchiveTurn, Turn::Fn::ConvIndexCap},
	State::Struct::AphroditeState,
};

/// Context-engine archiving entry (pre-Rust port of `_engine.py`
/// `AphroditeContextEngine`).
///
/// Metric (documented per the pre-Rust-port brief): the dylib never receives
/// the raw conversation message list (Hermes holds it) and has no token
/// counts or context length, so the closest available message-count proxy is
/// the conversation index itself - `conv_index.len()` is the number of
/// archived turns (each turn contributes at least one user + one assistant
/// message in the real conversation). The "message-count fill ratio" is
/// `conv_index.len() * 100 / CONV_INDEX_CAP`: the index's own eviction cap is
/// the only "full" reference the dylib has.
///
/// When the engine is enabled and the session crosses `EngineThresholdPct`
/// (fill ratio at or above the percent; 100+ = engine off, 0 = never, both
/// pre-Rust `should_compress` semantics) with at least `EngineMinimumMsgs`
/// entries, the middle turns of the index - keeping exactly
/// `EngineProtectFirst` at the start and `EngineProtectLast` at the end,
/// with the pre-Rust middle-size floor of ≥ 3 - are packed as one JSON
/// payload, stored in the inline/CCR store under a BLAKE3 key
/// (`headroom_core::ccr::compute_key`), and the middle is REPLACED in
/// `ConvIndex` by a single marker entry recorded via `ArchiveTurn` so the
/// model sees a compact history (head + marker + tail). The returned string
/// is the CCR marker plus the rendered `EngineOffload` prompt (hash + tail)
/// for the caller to inject into the live conversation.
///
/// `None` = nothing archived (engine off / below threshold / below min_msgs /
/// middle too small) with zero mutation - byte-identical behavior when
/// disabled.
pub fn EngineMaybeArchive(State:&mut AphroditeState) -> Option<String> {
	// Engine off (config), or threshold 0 (never) / 100+ (documented
	// engine-off escape hatch). All gates run before any mutation.
	match !State.ContextEngineEnabled || State.EngineThresholdPct == 0 || State.EngineThresholdPct >= 100 {
		true => {
			// DEV-ONLY debug instrumentation (crate::debug): engine decision.
			crate::Debug::Engine::Skip::Fn::Fn(State, "engine_off_or_threshold_gate");
			return None;
		},
		false => (),
	}

	let Total = State.ConvIndex.len();

	match Total < State.EngineMinimumMsgs {
		true => {
			crate::Debug::Engine::Skip::Fn::Fn(State, "below_min_msgs");
			return None;
		},
		false => (),
	}
	// Fill ratio: archived-turn count against the index's own cap.
	let FillPct = ((Total as u64 * 100) / ConvIndexCap as u64).min(100);

	match FillPct < State.EngineThresholdPct {
		true => {
			crate::Debug::Engine::Skip::Fn::Fn(State, "fill_below_threshold");
			return None;
		},
		false => (),
	}
	// Middle slice: exactly protect_first from the start, protect_last from
	// the end; pre-Rust `compress()` refuses a middle shorter than 3.
	let HeadN = State.EngineProtectFirst;
	let TailN = State.EngineProtectLast;
	let MiddleN = Total.saturating_sub(HeadN).saturating_sub(TailN);

	match MiddleN < 3 {
		true => {
			crate::Debug::Engine::Skip::Fn::Fn(State, "middle_too_small");
			return None;
		},
		false => (),
	}
	// Sorted turns, oldest first (same ordering as `GetConvIndex`).
	let mut Turns:Vec<usize> = State.ConvIndex.keys().copied().collect();
	Turns.sort_unstable();
	let End = Total.saturating_sub(TailN);
	let MiddleKeys:Vec<usize> = Turns[HeadN..End].to_vec();

	// Pack the REAL middle entries (recorded turn → hash/summary/size) as one
	// compact JSON payload - never fabricated message content.
	let PackedEntries:Vec<serde_json::Value> = MiddleKeys
		.iter()
		.filter_map(|t| {
			State.ConvIndex.get(t).map(
				|(hash, Summary, Size)| serde_json::json!({"turn": t, "hash": hash, "summary": Summary, "size": Size}),
			)
		})
		.collect();
	let Packed = match serde_json::to_string(&PackedEntries) {
		Ok(p) => p,
		Err(e) => {
			tracing::warn!(error = %e, "engine_maybe_archive: packing middle turns failed; skipping");
			return None;
		},
	};
	let hash = headroom_core::ccr::compute_key(Packed.as_bytes());
	State.InlineStorePut(hash.clone(), Packed.clone());
	let Preview = Packed.chars().take(120).collect::<String>();
	let Marker = crate::Marker::Format::Entry::CCRMarker(&hash, "context", Packed.len(), &Preview, None, None, None);
	let Rendered = crate::Flow::Prompt::Render::Fn::RenderPrompt(
		State,
		"engine_offload",
		&[("hash", &hash), ("tail", &TailN.to_string())],
	);
	let Prompt = format!("{Marker}\n{Rendered}");

	// Replace the archived middle with the marker entry: drop the middle
	// turns from the index, then record the new archive at the current turn.
	MiddleKeys.iter().for_each(|t| {
		State.ConvIndex.remove(t);
	});
	let Summary = format!("engine offload: {MiddleN} middle turns archived (keep {HeadN}/{TailN})");
	ArchiveTurn(State, &hash, &Summary, Packed.len());
	State.RecordMarker(crate::State::Marker::Struct::MarkerEntry {
		hash:hash.clone(),
		CCRType:"context".into(),
		Size:Packed.len(),
		Preview,
		Turn:State.TurnCounter,
		Center:None,
		meta:None,
	});
	// DEV-ONLY debug instrumentation (crate::debug): archive outcome.
	crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::EngineArchives);
	crate::Debug::Log::At::Fn::Fn(
		State.DebugLevel,
		"engine_archive",
		&[
			("hash", hash.clone()),
			("middle", MiddleN.to_string()),
			("head", HeadN.to_string()),
			("tail", TailN.to_string()),
			("size", Packed.len().to_string()),
		],
	);
	tracing::info!(
		hash,
		Middle = MiddleN,
		Size = Packed.len(),
		"context_engine: archived {} middle turns to CCR:{} ({} bytes)",
		MiddleN,
		hash,
		Packed.len()
	);
	Some(Prompt)
}
