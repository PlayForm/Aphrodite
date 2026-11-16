//! The context-engine archiving entry.
//!
//! Decides from the conversation index alone whether to compact the middle of the
//! history: the dylib never receives the raw message list (Hermes holds it) and
//! has no token counts, so the index's own cap is the only "full" reference
//! available. The decision is a value (`Assess/`), the work it names is a prepared
//! plan (`Plan/`), and this leaf is the interpreter that runs one or reports the
//! other.

use crate::{
	Session::Engine::{
		Apply::Fn::Fn as ApplyPlan,
		Assess::{Enum::Enum as Assessment, Fn::Fn as Assess},
	},
	State::Struct::AphroditeState,
};

/// Context-engine archiving entry (pre-Rust port of `_engine.py`
/// `AphroditeContextEngine`).
///
/// When the engine is enabled and the session crosses `EngineThresholdPct` (fill
/// ratio at or above the percent; 100+ = engine off, 0 = never, both pre-Rust
/// `should_compress` semantics) with at least `EngineMinimumMsgs` entries, the
/// middle turns of the index - keeping exactly `EngineProtectFirst` at the start
/// and `EngineProtectLast` at the end, with the pre-Rust middle-size floor of 3 -
/// are packed as one JSON payload, stored under a BLAKE3 key
/// (`headroom_core::ccr::compute_key`), and the middle is REPLACED in `Turn::Index`
/// by a single marker entry recorded via `Archive`, so the model sees a
/// compact history (head + marker + tail). The returned string is the CCR marker
/// plus the rendered `EngineOffload` prompt (hash + tail) for the caller to
/// inject into the live conversation.
///
/// `None` = nothing archived, with zero mutation - byte-identical behaviour when
/// disabled, because the refusal is decided before the plan is applied.
pub fn Fn(State:&mut AphroditeState) -> Option<String> {
	match Assess(State) {
		Assessment::Skip(Reason) => {
			// DEV-ONLY debug instrumentation (crate::debug): engine decision.
			crate::Debug::Engine::Skip::Fn::Fn(State, Reason.Text());

			None
		},
		Assessment::Archive(Plan) => ApplyPlan(State, &Plan),
	}
}
