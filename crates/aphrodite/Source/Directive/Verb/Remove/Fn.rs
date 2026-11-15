//! The `remove` action.

use crate::State::Struct::AphroditeState;

/// Deactivate `name`.
///
/// Silent for a name that was not active, and silent for a name that is not loaded at
/// all: removal is the "make sure this is off" operation, and both cases already satisfy
/// it. `retain` is the whole implementation - it removes every occurrence, which is what
/// makes it idempotent even if a duplicate ever reached the list.
pub(crate) fn Fn(State:&mut AphroditeState, name:&str) -> serde_json::Value {
	State.ActiveDirectives.retain(|Active| Active != name);
	State.ManualDirectiveTurn = Some(State.TurnCounter);

	serde_json::json!({"active": &State.ActiveDirectives})
}
