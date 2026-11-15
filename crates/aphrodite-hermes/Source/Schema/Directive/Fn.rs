//! `aphrodite_directive` - inspect or change the active behavioral directives.
//!
//! The longest schema in the set, because the six actions have genuinely
//! different return shapes and two of them (`add` vs `load`) differ only in how
//! they treat an unknown name. Collapsing that distinction into one sentence
//! produced real misuse, so it is spelled out per action.

use serde_json::json;

/// The `aphrodite_directive` tool schema.
pub(crate) fn Fn() -> serde_json::Value {
	json!({
		"name": "aphrodite_directive",
		"description": "Inspect or change the active behavioral directives. \
			Directives are short instruction files injected into every turn through \
			pre_llm_call - \"focus\" for terse, minimal-tool execution, \"explore\" for \
			broad reading, \"lazy\" for defer-until-needed execution, and whatever else ships in directives/. \
			`list` returns {available, active, ephemeral: [{name, inline, \
			expires_after_turn}]}; `swap` replaces the whole active set with one \
			directive and returns {swapped, active}; `add` appends to the set (silent \
			if already active) and returns {active}; `load` activates a directive from \
			the available set on demand and returns {loaded, active} (errors on an \
			unknown name, unlike `add`); `remove` drops one and returns {active}; \
			`reset` clears actives, ephemerals and the manual latch, handing control \
			back to automatic selection. An unknown directive name returns {error} \
			and changes nothing.",
		"parameters": {
			"type": "object",
			"properties": {
				"action": {
					"type": "string",
					"enum": ["list", "swap", "add", "load", "remove", "reset"],
					"default": "list",
					"description": "What to do. Defaults to `list`. `load` activates a directive just-in-time (lazy activation)."
				},
				"name": {
					"type": "string",
					"description": "Directive name. Required for swap, add, load and remove; ignored by list and reset."
				}
			},
			"additionalProperties": false
		}
	})
}
