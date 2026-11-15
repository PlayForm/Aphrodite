//! `aphrodite_files` - the file paths this session has touched.
//!
//! Named for the single tool family it describes. The `tool` field in each row
//! is the FIRST tool that referenced the path, not the last, which is what makes
//! the list read as a history rather than a set.

use serde_json::json;

use crate::Schema::NoParams;

/// The `aphrodite_files` tool schema.
pub(crate) fn Fn() -> serde_json::Value {
	json!({
		"name": "aphrodite_files",
		"description": "List file paths touched this session. \
			Populated as tool hooks observe reads, writes and prefetches, so it doubles \
			as a record of what has already been looked at. Takes no arguments. \
			Returns {total, files: [{path, tool}]} where `tool` is the tool that first \
			referenced the path.",
		"parameters": NoParams::Fn()
	})
}
