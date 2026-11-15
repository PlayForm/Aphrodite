//! Extract a file path from tool output.

/// Extract file path from tool output - heuristic.
///
/// Feeds the `ReferencedFiles` catalog stat and, for write-style tools, the
/// `WrotePath` column of the tool-event ring. It is a heuristic on the FIRST
/// line only: a path that does not start at the beginning of the first line is
/// missed rather than guessed, because a wrong path in the catalog is worse
/// than a missing one.
pub(crate) fn Fn(Content:&str, Tool:&str) -> Option<String> {
	match Tool {
		"read_file" | "write_file" | "patch" => {
			// First line often contains path
			Content.lines().next().and_then(|Line| {
				let Line = Line.trim();
				if Line.starts_with('/') || Line.starts_with("./") {
					Some(Line.to_string())
				} else {
					None
				}
			})
		},
		// `search_files` output is conventionally a `path:line:match` (grep-
		// style) list; the file being referenced is whatever's before the
		// first `:` on the first result line (F10: this tool is listed in
		// `state.file_tools` but was previously never actually matched here,
		// so every search result silently went untracked).
		"search_files" => {
			Content.lines().next().and_then(|Line| {
				let path = Line.split(':').next().unwrap_or("").trim();
				if path.starts_with('/') || path.starts_with("./") {
					Some(path.to_string())
				} else {
					None
				}
			})
		},
		_ => None,
	}
}
