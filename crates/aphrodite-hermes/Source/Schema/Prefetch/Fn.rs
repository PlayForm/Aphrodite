//! `aphrodite_prefetch` - read files now and store them as markers.
//!
//! The name suggests asynchrony and the implementation is synchronous; the
//! description has to say so, because a caller that assumed a background read
//! would use a hash before it existed. The budget warning is equally load-bearing:
//! a batch larger than the inline-store budget evicts its own earlier entries.

use serde_json::json;

/// The `aphrodite_prefetch` tool schema.
pub(crate) fn Fn() -> serde_json::Value {
	json!({
		"name": "aphrodite_prefetch",
		"description": "Read files now and store them as CCR markers. \
			Despite the name this is synchronous: every path is read and compressed \
			before the call returns, so each returned hash is immediately resolvable \
			through aphrodite_retrieve. Batch the files you expect to need next instead \
			of reading them one at a time - one call, one marker per file, no file \
			bodies in context. Oversized files are skipped rather than truncated. \
			Returns {total, loaded, skipped_size, missing, inline_store_bytes, \
			inline_store_byte_budget, results: [{path, status: \
			\"loaded\"|\"skipped\"|\"missing\", hash, type, size, preview, reason}]}. \
			Watch inline_store_bytes against its budget: a batch larger than the budget \
			evicts older entries, including ones from the same batch.",
		"parameters": {
			"type": "object",
			"properties": {
				"paths": {
					"type": "array",
					"items": {"type": "string"},
					"minItems": 1,
					"description": "File paths to read and store. Absolute paths are safest; relative paths resolve against the session working directory."
				}
			},
			"required": ["paths"],
			"additionalProperties": false
		}
	})
}
