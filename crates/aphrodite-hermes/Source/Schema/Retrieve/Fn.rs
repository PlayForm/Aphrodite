//! `aphrodite_retrieve` - expand a marker, or read a file, to full content.
//!
//! The hash-vs-path exclusivity is expressed in prose, not as an `anyOf` union:
//! the sanitizers strip top-level unions, so a union here would silently become
//! "anything goes" at the provider boundary.

use serde_json::json;

/// The `aphrodite_retrieve` tool schema.
pub(crate) fn Fn() -> serde_json::Value {
	json!({
		"name": "aphrodite_retrieve",
		"description": "Expand a CCR marker, or read a file, to full content. \
			Supply `hash` to resolve a marker recorded this session (nested markers are \
			expanded recursively), or `path` to read a file straight from disk. Give one \
			or the other - if both are present `path` wins. `path` reads are confined to \
			the current workspace and capped at 10 MiB, so paths outside it are refused. \
			`query` narrows the result to matching lines, which is the cheap way to poke \
			at a large entry without pulling all of it into context. \
			Returns {found: true, source: \"ccr\"|\"path\", hash|path, content} on success, \
			{found: false, error} when the hash is unknown or the read is refused.",
		"parameters": {
			"type": "object",
			"properties": {
				"hash": {
					"type": "string",
					"pattern": "^[0-9a-fA-F]{8,64}$",
					"description": "Full hex hash from a `<<<CCR:hash|type|size>>>` marker. Truncated hashes do not resolve - exact match only."
				},
				"query": {
					"type": "string",
					"description": "Case-insensitive substring; only lines containing it are returned."
				},
				"path": {
					"type": "string",
					"description": "Workspace-relative or absolute file path to read directly, bypassing CCR."
				}
			},
			"additionalProperties": false
		}
	})
}
