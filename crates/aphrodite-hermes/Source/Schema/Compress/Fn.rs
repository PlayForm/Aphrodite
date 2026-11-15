//! `aphrodite_compress` - store content, get a resolvable marker back.
//!
//! The lead sentence and every parameter description here are read verbatim by
//! the model; the first sentence is also the whole deferred-catalog line. Do not
//! reword them as part of a refactor.

use serde_json::json;

/// The `aphrodite_compress` tool schema.
pub(crate) fn Fn() -> serde_json::Value {
	json!({
		"name": "aphrodite_compress",
		"description": "Store content in CCR and get a marker back. \
			Hashes the content, classifies it (or trusts the `type` hint), keeps it in \
			the session store, and hands back a resolvable `<<<CCR:hash|type|size>>>` \
			marker - park bulky text here instead of carrying it in context, then pull \
			it back with aphrodite_retrieve when you actually need it. Storage is \
			in-process and sub-millisecond; markers stay resolvable for the life of the \
			session but do not survive a process restart. \
			Returns {hash, type, size, preview, marker}.",
		"parameters": {
			"type": "object",
			"properties": {
				"content": {
					"type": "string",
					"minLength": 1,
					"description": "Exact text to store. Round-trips byte-for-byte through aphrodite_retrieve."
				},
				"type": {
					"type": "string",
					"enum": ["code", "log", "diff", "error", "json", "build_output", "text"],
					"description": "Content-type hint that steers the preview and compression profile. Omit to auto-detect; \"text\" is treated as no hint."
				},
				"_ccr_center": {
					"type": "string",
					"description": "Optional label carried alongside the marker and echoed back by aphrodite_catalog - useful for tagging what a stored blob was for."
				}
			},
			"required": ["content"],
			"additionalProperties": false
		}
	})
}
