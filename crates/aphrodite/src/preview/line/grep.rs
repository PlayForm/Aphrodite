//! grep/ripgrep hit-line predicates (`path:line:match` / `path:match`).

/// Split a `path:line:...` candidate; `None` when the path is missing,
/// whitespace-containing, or empty, or the middle field is not a line number.
/// Shared by `is_grep_line` and `is_search_line` (Phase 4: the search
/// predicate is `is_grep_line` minus the file-ish path check).
fn split_path_line(line:&str) -> Option<(&str, &str)> {
	let mut it = line.splitn(3, ':');
	let path = match it.next() {
		Some(p) if !p.trim().is_empty() && !p.contains(' ') => p,
		_ => return None,
	};
	match it.next() {
		// `path:line:...` - middle is a line number, third segment exists.
		Some(mid) if mid.chars().all(|c| c.is_ascii_digit()) && !mid.is_empty() && it.next().is_some() => {
			Some((path, mid))
		},
		_ => None,
	}
}

/// True when a line looks like a grep/ripgrep hit: `path:line:match` (with a
/// numeric line field) or `path:match` where the path has a file-ish shape.
pub(crate) fn is_grep_line(line:&str) -> bool {
	let (path, _mid) = match split_path_line(line) {
		Some(p) => p,
		None => return false,
	};
	// Path should look file-ish: contain a `/` or a `.ext`.
	path.contains('/') || path.contains('.')
}

/// True for a `path:line:` search hit (the old SEARCH_RE shape): the same
/// splitn core as `is_grep_line`, WITHOUT the file-ish path requirement.
///
/// Phase-6 refinement: a `path` that starts with a digit and contains no
/// `/` or `.` is a timestamp (ISO/syslog `2026-09-17T10:00:00Z ...`), not a
/// search path - without this guard the search detector would hijack the
/// log arm's timestamped lines (battery pin
/// `test_preview_log_surfaces_error_signal_or_tail`).
pub(crate) fn is_search_line(line:&str) -> bool {
	match split_path_line(line) {
		Some((path, _)) => {
			path.contains('/') || path.contains('.') || !path.starts_with(|c:char| c.is_ascii_digit())
		},
		None => false,
	}
}