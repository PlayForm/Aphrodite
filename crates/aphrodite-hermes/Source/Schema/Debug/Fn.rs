//! `aphrodite_debug` - the dev-only per-session debug toggle.
//!
//! Compiled only under `debug_assertions`: a release dylib that registers this
//! tool fails `hermes plugins validate` rule 6 as an undeclared tool, because the
//! manifest declares exactly 13 production tools.

use serde_json::json;

/// The `aphrodite_debug` tool schema.
///
/// Dev builds only - the whole module is gated, so this never reaches a release
/// catalogue.
pub(crate) fn Fn() -> serde_json::Value {
	json!({
		"name": "aphrodite_debug",
		"description": "Toggle per-session debug output on or off (Rust-side only). \
			When on, every compressed tool result in this session tree (this session \
			plus its subagents/delegated tasks) gets a `[aphrodite-debug ...]` line \
			prepended before the CCR marker; other sessions stay quiet. The flag is a \
			file in the runtime home, scoped to the ROOT session id - subagents \
			resolve to the same root and inherit the toggle. \
			Returns {status, debug, session, flag}.",
		"parameters": {
			"type": "object",
			"properties": {
				"on": {
					"type": "boolean",
					"description": "true = enable debug for this session tree, false = disable. Omitted defaults to true."
				}
			},
			"additionalProperties": false
		}
	})
}
