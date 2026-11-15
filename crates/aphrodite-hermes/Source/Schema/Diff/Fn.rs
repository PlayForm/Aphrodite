//! `aphrodite_diff` - the per-turn history of what was compressed.
//!
//! The only schema whose entries are guaranteed resolvable: a turn only appears
//! here after `post_llm_call` archived it, so every listed hash can still be
//! expanded through `aphrodite_retrieve`.

use serde_json::json;

use crate::Schema::NoParams;

/// The `aphrodite_diff` tool schema.
pub(crate) fn Fn() -> serde_json::Value {
	json!({
		"name": "aphrodite_diff",
		"description": "Per-turn history of what was compressed. \
			Walks the archived conversation index in turn order, one entry per turn that \
			produced a compression. Takes no arguments. \
			Returns {total, turns: [{turn, hash, summary, size}]}; each `hash` is \
			resolvable through aphrodite_retrieve.",
		"parameters": NoParams::Fn::Fn()
	})
}
