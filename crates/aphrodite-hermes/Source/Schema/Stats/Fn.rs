//! `aphrodite_stats` - session counters plus a live proxy poll.
//!
//! The one schema whose return shape names both halves of the system, which is
//! why its description is long: a caller that does not know `alive: false` means
//! "nothing answered in 400ms, the plugin still works" reads it as a failure.

use serde_json::json;

use crate::Schema::NoParams;

/// The `aphrodite_stats` tool schema.
pub(crate) fn Fn() -> serde_json::Value {
	json!({
		"name": "aphrodite_stats",
		"description": "Session and proxy health counters for the CCR engine. \
			One call covers both halves of the system: session state held by the dylib, \
			and a live TCP poll of the two proxy ports. Takes no arguments. \
			Returns {version, engine, inline_entries, markers, referenced_files, \
			archived_turns, turn, engine_enabled, threshold_pct, tool_threshold, \
			terminal_threshold, proxies: {token: {port, alive}, cache: {port, alive}}}. \
			`alive: false` means nothing answered on that port within 400ms - the plugin \
			still works, it just is not proxying.",
		"parameters": NoParams::Fn()
	})
}
