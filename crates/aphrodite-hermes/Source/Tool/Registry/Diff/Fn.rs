//! `aphrodite_diff` - the archived per-turn compression history.

use Tool::Dispatch::Fn::ToolHandler;

/// The `aphrodite_diff` handler.
///
/// Reads the conversation index only - it does not walk the inline store, so a
/// marker that was never archived by `post_llm_call` does not appear here even
/// though `aphrodite_retrieve` can still resolve it.
pub(crate) fn Fn() -> (&'static str, ToolHandler) {
	("aphrodite_diff", |_args| {
		crate::Session::With::Fn::with_shared(|state| {
			let turns = aphrodite::Session::GetConvIndex(state);
			serde_json::json!({"total": turns.len(), "turns": turns})
		})
	})
}
