//! The context-engine trigger and the section it produces.

use crate::{
	Flow::Engine::Offload::{Archive::Fn::Fn as Archive, Section::Fn::Fn as Wrap},
	State::Struct::AphroditeState,
};

/// Context-engine offload section (pre-Rust port of `_engine.py` `compress`):
/// while the context engine is enabled (`state.context_engine_enabled`), ask it
/// whether it archived middle messages this turn
/// ([`crate::Session::Engine::Fn::EngineMaybeArchive`]); when it returns the
/// rendered `EngineOffload` prompt (marker + guidance), inject it so the model
/// sees the compacted history. Disabled, or enabled with the engine returning
/// `None` (nothing archived), = no injection, byte-identical current behavior.
///
/// Unlike the auto-expand line it is counted in the always-survive tier when
/// present: the archive already mutated the conversation (session.rs owns
/// that), so the explanation must not be the first section dropped under budget
/// pressure.
///
/// Empty string = no section this turn.
pub(crate) fn Fn(State:&mut AphroditeState) -> String { Wrap(Archive(State)) }
