//! Infer the path a write-style tool wrote.

use crate::Hook::Path::Fn::Fn as ExtractFilePath;

/// Infer the written path for a write-style tool (P2/T6, feeds P11). Reads a
/// `path`/`file` arg for `write_file`/`patch`; falls back to the first-line
/// path heuristic used by `ExtractFilePath`.
///
/// The arg wins over the output heuristic because a `patch` reply often opens
/// with a diff header rather than the path, while the args always name the
/// file the agent asked to touch. It returns `None` for every other tool - a
/// guessed write path would pollute the very signal it feeds.
pub(crate) fn Fn(ToolName:&str, Args:Option<&serde_json::Value>, Content:&str) -> Option<String> {
	match !matches!(ToolName, "write_file" | "patch") {
		true => {
			return None;
		},
		false => {},
	}
	let ArgPath = Args.and_then(|v| {
		["path", "file", "file_path", "filename"].into_iter().find_map(|Key| {
			v.get(Key)
				.and_then(|x| x.as_str())
				.filter(|p| !p.is_empty())
				.map(|p| p.to_string())
		})
	});
	match ArgPath {
		Some(Path) => Some(Path),
		None => ExtractFilePath(Content, ToolName),
	}
}
