//! The `load` action.

use crate::State::Struct::AphroditeState;
use super::super::{Push, Unknown};

/// Activate `name` on demand, refusing a name that is not loaded.
///
/// Unlike `add`, this is an explicit, intentional activation - a lazy load - so a typo
/// surfaces as an error instead of being swallowed, and the reply has its own shape
/// (`loaded`) so a caller can tell it apart from a pre-seeded `add`. Idempotent when the
/// directive is already active.
pub(crate) fn Fn(State:&mut AphroditeState, name:&str) -> serde_json::Value {
	let Some(name) = State.Directives.contains_key(name).then_some(name) else {
		return Unknown::Fn::Fn(name);
	};

	Push::Fn::Fn(&mut State.ActiveDirectives, name);
	State.ManualDirectiveTurn = Some(State.TurnCounter);

	serde_json::json!({"loaded": name, "active": &State.ActiveDirectives})
}
