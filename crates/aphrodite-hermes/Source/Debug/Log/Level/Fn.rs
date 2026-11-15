//! The effective debug level for a session: env level OR the flag toggle.
//!
//! Two independent switches exist because they answer different questions. The
//! env var is an operator decision made at launch and is a LEVEL (0, 1, 2); the
//! flag file is a per-session-tree decision made mid-run and is a boolean. This
//! leaf is the single place they are reconciled.

use Debug::Flag::Resolve::Fn::Fn as enabled_for;

/// Effective debug level for a session: the env-gated level (captured at
/// launch) wins; otherwise the session flag-file toggle applies (level 1).
///
/// It does not add the two: an operator who asked for level 2 keeps level 2
/// even with the flag on, because the flag can only ever mean "verbose", never
/// "more verbose than the operator asked for".
pub(crate) fn Fn(state:&aphrodite::State::AphroditeState, session:&str) -> u8 {
	if state.DebugLevel > 0 {
		state.DebugLevel
	} else if enabled_for(session) {
		1
	} else {
		0
	}
}
