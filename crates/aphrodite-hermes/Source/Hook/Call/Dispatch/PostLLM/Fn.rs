//! The `post_llm_call` arm: end the turn and archive it.
//!
//! Delegates to `Hook::post_llm_call`, NOT a bare `next_turn` (report 06
//! F11/T13). This is the process-global dispatch path the Hermes Python plugin
//! actually calls on every turn, so calling `next_turn` directly here bypassed
//! the turn-archive step entirely and `conv_index` stayed empty in real usage
//! even after `Hook::post_llm_call` was wired - `aphrodite_diff` then reported
//! zero turns forever.

use aphrodite::State::Struct::AphroditeState;

/// Advance the turn counter and archive the turn's marker.
pub(crate) fn post_llm_call(state:&mut AphroditeState) -> serde_json::Value {
	aphrodite::Hook::LLM::Post::Fn::Fn(state);
	serde_json::Value::Null
}
