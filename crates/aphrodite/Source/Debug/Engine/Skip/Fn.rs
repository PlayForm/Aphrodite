//! `engine_skip`: why the engine archived nothing this turn.

use crate::{Debug::Log::At::Fn::Fn as LogAt, State::AphroditeState};

/// DEV-ONLY: engine skip line - the context-engine decision when nothing
/// was archived this turn.
///
/// The level comes from the STATE, not the process mirror: the engine runs on
/// the session that owns it, and a skip reason recorded under another
/// session's level would be the one line an investigation cannot trust.
pub fn Fn(State:&AphroditeState, Reason:&str) {
	if State.DebugLevel == 0 {
		return;
	}
	LogAt(State.DebugLevel, "engine_skip", &[("reason", Reason.to_string())]);
}
