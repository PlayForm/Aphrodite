//! Extract a file path from tool output.

use crate::Hook::Path::{Resolved::Struct::Struct as Resolved, Source::Enum::Enum as Source};

/// Extract a file path from tool output - heuristic.
///
/// Feeds the `File::Recent` catalog stat. It is a heuristic on the FIRST
/// line only: a path that does not start at the beginning of the first line is
/// missed rather than guessed, because a wrong path in the catalog is worse
/// than a missing one. The answer carries `Source::Output` so a consumer never
/// mistakes it for an argument.
pub(crate) fn Fn(Content:&str, Tool:&str) -> Option<Resolved> {
	let Path = match Tool {
		// First line often contains path.
		"read_file" | "write_file" | "patch" => {
			Content
				.lines()
				.next()
				.map(str::trim)
				.filter(|Line| Line.starts_with('/') || Line.starts_with("./"))
		},
		// `search_files` output is conventionally a `path:line:match` (grep-
		// style) list; the file being referenced is whatever is before the
		// first `:` on the first result line.
		"search_files" => {
			Content
				.lines()
				.next()
				.map(|Line| Line.split(':').next().unwrap_or("").trim())
				.filter(|Path| Path.starts_with('/') || Path.starts_with("./"))
		},
		_ => None,
	}?;

	Some(Resolved { Path:Path.to_string(), Source:Source::Output })
}
