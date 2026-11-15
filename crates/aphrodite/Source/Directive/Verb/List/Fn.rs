//! The `list` action.

use crate::State::Struct::AphroditeState;

/// Report every loaded directive, the active set and the ephemeral entries.
///
/// `available` is SORTED: `HashMap` iteration order is nondeterministic, which made the
/// result flake between `["focus","lazy"]` and `["lazy","focus"]` across runs. The active
/// list keeps its activation order, which is the order the context injector renders in,
/// and the ephemeral entries carry their expiry so the nudge/TTL mechanism is observable
/// (P3/T10).
pub(crate) fn Fn(State:&AphroditeState) -> serde_json::Value {
	let Ephemeral:Vec<serde_json::Value> = State
		.EphemeralDirectives
		.iter()
		.map(|Ephemeral| {
			serde_json::json!({
				"name": Ephemeral.name,
				"inline": Ephemeral.inline,
				"expires_after_turn": Ephemeral.ExpiresAfterTurn,
			})
		})
		.collect();

	let mut Available:Vec<&String> = State.Directives.keys().collect();
	Available.sort();

	serde_json::json!({
		"available": Available,
		"active": &State.ActiveDirectives,
		"ephemeral": Ephemeral,
	})
}
