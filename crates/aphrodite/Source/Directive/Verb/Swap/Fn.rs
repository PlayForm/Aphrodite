//! The `swap` action.

use crate::State::Struct::AphroditeState;
use super::super::{Push, Unknown};

/// Replace the whole active set with `name`.
///
/// A swap of an unknown name is refused, because the caller asked for exactly that one
/// directive and a silent empty set would look like success.
pub(crate) fn Fn(State:&mut AphroditeState, name:&str) -> serde_json::Value {
	// The name is usable only when it is loaded; `then_some` keeps the refusal a value
	// rather than a test the caller has to interpret.
	let Some(name) = State.Directive.Loaded.contains_key(name).then_some(name) else {
		return Unknown::Fn::Fn(name);
	};

	State.Directive.Active.clear();
	Push::Fn::Fn(&mut State.Directive.Active, name);
	State.Directive.Manual = Some(State.Turn.Counter);

	serde_json::json!({"swapped": name, "active": &State.Directive.Active})
}
