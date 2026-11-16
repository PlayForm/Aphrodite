//! `aphrodite_files` - file paths referenced this session.

use crate::Tool::Dispatch::Fn::ToolHandler;

/// The `aphrodite_files` handler.
///
/// Each row carries the tool that FIRST referenced the path, so the list reads as
/// a history of discovery rather than a set - the same path seen twice keeps its
/// original `tool`.
pub(crate) fn Fn() -> (&'static str, ToolHandler) {
	("aphrodite_files", |_Args| {
		crate::Session::With::Fn::with_shared(|State| {
			let Files:Vec<serde_json::Value> = State
				.ReferencedFiles
				.iter()
				.map(|(Path, Tool)| serde_json::json!({"path": Path, "tool": Tool}))
				.collect();

			serde_json::json!({"total": Files.len(), "files": Files})
		})
	})
}
