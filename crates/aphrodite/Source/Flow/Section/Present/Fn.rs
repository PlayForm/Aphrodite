//! Whether the never-drop nudge block exists this turn.
//!
//! The floor the budget may never cross counts present always-survive sections, so
//! "present" has to mean the same thing here as it does in the assembler: an inline
//! nudge that has not expired.

use crate::State::Struct::AphroditeState;

/// `true` when at least one live inline nudge is queued.
pub(crate) fn Fn(State:&AphroditeState) -> bool {
	State.EphemeralDirectives.iter().any(|Entry| {
		Entry.inline.is_some() && Entry.ExpiresAfterTurn.is_none_or(|Expires| Expires >= State.TurnCounter)
	})
}
