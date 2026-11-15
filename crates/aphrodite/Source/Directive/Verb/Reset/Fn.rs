//! The `reset` action.

use crate::State::Struct::AphroditeState;

/// Return to auto mode: clear the actives, the ephemeral nudges and the latch.
///
/// The latch is part of the reset and not an afterthought (P3/P6, see T10/T20): a reset
/// that left `ManualDirectiveTurn` set would keep phase-aware auto-swaps switched off
/// while the user believes they turned directives off, which is the opposite of what they
/// asked for.
pub(crate) fn Fn(State:&mut AphroditeState) -> serde_json::Value {
	State.ActiveDirectives.clear();
	State.EphemeralDirectives.clear();
	State.ManualDirectiveTurn = None;

	serde_json::json!({"active": &State.ActiveDirectives})
}
