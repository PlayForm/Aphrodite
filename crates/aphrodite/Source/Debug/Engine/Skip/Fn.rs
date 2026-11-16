//! `engine_skip`: why the engine archived nothing this turn.

use crate::{Debug::Log::At::Fn::Fn as LogAt, State::Struct::AphroditeState};

/// DEV-ONLY: engine skip line - the context-engine decision when nothing
/// was archived this turn.
///
/// The level comes from the STATE, not the process mirror: the engine runs on
/// the session that owns it, and a skip reason recorded under another
/// session's level would be the one line an investigation cannot trust.
///
/// The level-0 gate is a fast path in front of `LogAt`'s own gate: a quiet
/// session formats nothing and touches no path.
pub fn Fn(State:&AphroditeState, Reason:&str) {
	// A level of 0 is "off", which is exactly what `NonZeroU8` refuses to hold: the
	// level is the value that logs, and the reason is formatted only for it.
	std::num::NonZeroU8::new(State.DebugLevel)
		.into_iter()
		.for_each(|Level| LogAt(Level.get(), "engine_skip", &[("reason", Reason.to_string())]));
}
