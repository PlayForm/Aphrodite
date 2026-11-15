//! `aphrodite_search` - find already-compressed entries by keyword or type.
//!
//! The description has to say what it does NOT do (search stored bodies), because
//! a model that reads "search" and gets a miss concludes the content is gone
//! rather than concluding it should fall back to the catalogue.

use serde_json::json;

/// The `aphrodite_search` tool schema.
pub(crate) fn Fn() -> serde_json::Value {
	json!({
		"name": "aphrodite_search",
		"description": "Find already-compressed entries by keyword or type. \
			Matches case-insensitively against each entry's preview line and its CCR \
			type - it does NOT search the full stored bodies, so treat a miss as \
			inconclusive and fall back to aphrodite_catalog. Newest first, capped at 20 \
			results. \
			Returns {query, total, results: [{hash, type, size, preview}]}.",
		"parameters": {
			"type": "object",
			"properties": {
				"query": {
					"type": "string",
					"description": "Keyword or phrase to match against preview text and CCR type."
				},
				"type": {
					"type": "string",
					"description": "Restrict results to one exact CCR type, such as source_code, terminal, build_output, diff, json, search or text."
				}
			},
			"required": ["query"],
			"additionalProperties": false
		}
	})
}
