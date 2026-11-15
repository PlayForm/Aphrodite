//! `path:line:` search-hit predicate.

use super::SplitPath;

/// True for a `path:line:` search hit.
///
/// The same split as `Hit::Fn`, WITHOUT the file-ish path requirement, so a
/// query result whose path has no extension is still a hit. One refinement
/// matters: a path starting with a digit and containing no `/` or `.` is a
/// timestamp (ISO or syslog `2026-09-17T10:00:00Z ...`), not a search path.
/// Without that guard the search detector hijacks the log arm's timestamped
/// lines and a log previews as a search result.
pub(crate) fn Fn(Line:&str) -> bool {
	match SplitPath::Fn(Line) {
		Some((path, _)) => path.contains('/') || path.contains('.') || !path.starts_with(|c:char| c.is_ascii_digit()),
		None => false,
	}
}
