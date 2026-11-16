//! `aphrodite_diff` - the archived per-turn compression history.

use crate::Tool::Dispatch::Fn::ToolHandler;

/// The `aphrodite_diff` handler.
///
/// Reads the conversation index only - it does not walk the inline store, so a
/// marker that was never archived by `post_llm_call` does not appear here even
/// though `aphrodite_retrieve` can still resolve it.
pub(crate) fn Fn() -> (&'static str, ToolHandler) {
	("aphrodite_diff", |_Args| {
		crate::Session::With::Fn::with_shared(|State| {
			let Turns = aphrodite::Session::Index::Fn::GetConvIndex(State);

			serde_json::json!({"total": Turns.len(), "turns": Turns})
		})
	})
}
