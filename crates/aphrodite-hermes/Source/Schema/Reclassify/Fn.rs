//! `aphrodite_reclassify` - re-detect type and preview for stored entries.
//!
//! A repair tool: it rewrites the catalogue row, never the stored bytes, which is
//! why the description has to say so - a caller that feared for its hashes would
//! not run it.

use serde_json::json;

/// The `aphrodite_reclassify` tool schema.
pub(crate) fn Fn() -> serde_json::Value {
	json!({
		"name": "aphrodite_reclassify",
		"description": "Re-detect type and preview for stored entries. \
			Re-runs classification against content already in the store and rewrites the \
			catalog entry in place, which repairs entries that were typed before a \
			classifier improvement or stored with a wrong hint. Content and hashes are \
			never modified, so existing markers keep resolving. \
			Returns {status: \"ok\", reclassified: <count>}.",
		"parameters": {
			"type": "object",
			"properties": {
				"hash": {
					"type": "string",
					"pattern": "^[0-9a-fA-F]{8,64}$",
					"description": "Reclassify only this entry. Omit to reclassify every entry in the session."
				}
			},
			"additionalProperties": false
		}
	})
}
