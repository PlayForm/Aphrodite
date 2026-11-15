//! `aphrodite_test` - the in-process smoke test of the compress/retrieve loop.
//!
//! Named `Smoke/` rather than `Test/` because `Schema/Test/` is this module's own
//! unit-test tree (contract 10: tests stay with their module), and two siblings
//! cannot share one name. The directory names the tool's JOB - a smoke test -
//! while the schema text still carries the real tool name.

use serde_json::json;

/// The `aphrodite_test` tool schema.
pub(crate) fn Fn() -> serde_json::Value {
	json!({
		"name": "aphrodite_test",
		"description": "Smoke-test the compress and retrieve round trip. \
			Compresses built-in samples in-process and checks each one comes back \
			byte-identical, then reports proxy health alongside. Use it to confirm the \
			engine is actually wired up before trusting a marker. \
			Returns {mode, status: \"ok\"|\"fail\", passed, total, \
			checks: [{type, hash, roundtrip}], proxies}. `status` is \"ok\" only when \
			every check round-tripped.",
		"parameters": {
			"type": "object",
			"properties": {
				"mode": {
					"type": "string",
					"enum": ["quick", "full"],
					"default": "quick",
					"description": "`quick` runs 1 sample (source code). `full` runs 3 (source code, build output with errors and warnings, JSON). Any unrecognized value behaves like `full`."
				}
			},
			"additionalProperties": false
		}
	})
}
