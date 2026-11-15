//! `aphrodite_rebuild` - report dylib version and proxy health.
//!
//! A report, not an action. The loaded dylib cannot safely replace itself
//! mid-session, so the description names the command an operator must run outside
//! the session instead of promising a rebuild it cannot perform.

use serde_json::json;

use crate::Schema::NoParams;

/// The `aphrodite_rebuild` tool schema.
pub(crate) fn Fn() -> serde_json::Value {
	json!({
		"name": "aphrodite_rebuild",
		"description": "Report dylib version and proxy health. \
			It does not rebuild or install anything - the loaded dylib cannot safely \
			replace itself mid-session - it reports the state an operator needs plus the \
			command to run outside the session. Takes no arguments. \
			Returns {status: \"ok\", version, proxies, hint}.",
		"parameters": NoParams::Fn::Fn()
	})
}
