//! `aphrodite_files` - file paths referenced this session.

use Tool::Dispatch::Fn::ToolHandler;

/// The `aphrodite_files` handler.
///
/// Each row carries the tool that FIRST referenced the path, so the list reads as
/// a history of discovery rather than a set - the same path seen twice keeps its
/// original `tool`.
pub(crate) fn Fn() -> (&'static str, ToolHandler) {
	("aphrodite_files", |_args| {
		crate::Session::With::Fn::with_shared(|state| {
			let files:Vec<serde_json::Value> = state
				.ReferencedFiles
				.iter()
				.map(|(path, tool)| serde_json::json!({"path": path, "tool": tool}))
				.collect();
			serde_json::json!({"total": files.len(), "files": files})
		})
	})
}
