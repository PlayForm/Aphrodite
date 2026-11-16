//! The `add` action.

use crate::State::Struct::AphroditeState;
use super::super::Push;

/// Activate `name` if it is loaded, silently doing nothing otherwise.
///
/// Silent by contract: `add` is the pre-seeding path, so an unknown name is a caller's
/// stale list rather than a user's typo, and the reply is the active set either way.
/// `load` exists precisely because that silence is wrong for an explicit activation.
///
/// The manual latch is set on EVERY `add`, including one that changed nothing: the caller
/// expressed a manual choice, and a manual choice must switch phase-aware auto-swaps off
/// (P6).
pub(crate) fn Fn(State:&mut AphroditeState, name:&str) -> serde_json::Value {
	let Known = State.Directive.Loaded.contains_key(name);

	Known
		.then_some(name)
		.into_iter()
		.for_each(|Name| Push::Fn::Fn(&mut State.Directive.Active, Name));

	State.Directive.Manual = Some(State.Turn.Counter);

	serde_json::json!({"active": &State.Directive.Active})
}
