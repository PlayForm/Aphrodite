//! The context-engine trigger and the explanation it injects.
//!
//! Three pieces, kept apart because they fail differently: the gate decides
//! whether the engine is consulted at all, the engine decides whether anything
//! was archived, and the wrapper decides how a Some/None answer becomes text.
//! Collapsing them would make "the engine ran and archived nothing"
//! indistinguishable from "the engine was never asked".

use crate::State::AphroditeState;

/// Context-engine trigger + offload section (pre-Rust port of `_engine.py`
/// `compress`): while the context engine is enabled
/// (`state.context_engine_enabled`), ask it whether it archived middle
/// messages this turn ([`crate::Session::engine_maybe_archive`]); when it
/// returns the rendered `EngineOffload` prompt (marker + guidance), inject
/// it so the model sees the compacted history. Disabled, or enabled with the
/// engine returning `None` (nothing archived), = no injection, byte-identical
/// current behavior.
///
/// Mirrors `AutoExpandGuidanceBlock` (gate on the state flag, section
/// injection through [`BuildTurnContext`]). Unlike the auto-expand line it
/// is counted in the always-survive tier when present: the archive already
/// mutated the conversation (session.rs owns that), so the explanation must
/// not be the first section dropped under budget pressure.
pub(crate) fn EngineOffloadBlock(State:&mut AphroditeState) -> String {
	EngineOffloadSectionFrom(EngineArchivePrompt(State))
}

/// Gate + trigger: call [`crate::Session::engine_maybe_archive`] only while
/// the context engine is enabled. Disabled => the engine is never consulted
/// (this is the trigger's only call site) and nothing is injected.
pub(crate) fn EngineArchivePrompt(State:&mut AphroditeState) -> Option<String> {
	if !State.ContextEngineEnabled {
		return None;
	}
	crate::Session::EngineMaybeArchive(State)
}

/// Wrap the engine's offload prompt into the injected section text; `None`
/// or an empty prompt injects nothing. Split out so the Some/None injection
/// semantics stay testable while `Session::engine_maybe_archive` is still the
/// pre-landed stub (Task 3 implements the body).
pub(crate) fn EngineOffloadSectionFrom(Prompt:Option<String>) -> String {
	match Prompt {
		Some(Text) if !Text.is_empty() => format!("[engine-offload: {Text}]"),
		_ => String::new(),
	}
}
