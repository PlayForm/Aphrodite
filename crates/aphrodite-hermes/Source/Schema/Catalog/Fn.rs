//! `aphrodite_catalog` - the complete inventory of recorded entries.
//!
//! The reliable fallback the search schema points at: it filters nothing, so a
//! half-remembered marker can always be found here. The two modes differ only in
//! how many fields each row carries.

use serde_json::json;

/// The `aphrodite_catalog` tool schema.
pub(crate) fn Fn() -> serde_json::Value {
	json!({
		"name": "aphrodite_catalog",
		"description": "List every CCR entry recorded this session. \
			The complete inventory, newest first - unlike aphrodite_search it filters \
			nothing, so this is the reliable way to find a marker you half-remember. \
			Returns {mode, total, turn, items: [...]}, where each item is \
			{hash, type, size, preview} in `toc` mode and additionally {turn, center} in \
			`full` mode.",
		"parameters": {
			"type": "object",
			"properties": {
				"mode": {
					"type": "string",
					"enum": ["full", "toc"],
					"default": "full",
					"description": "`full` includes turn and center per entry. `toc` is the compact table-of-contents form."
				}
			},
			"additionalProperties": false
		}
	})
}
