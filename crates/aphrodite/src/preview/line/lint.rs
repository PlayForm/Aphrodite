//! Linter issue-line predicate (ruff/eslint/clippy/flake8 shapes).

use crate::preview::line::{error::is_error_line, warning::is_warning_line};

/// True for a linter issue line: `path:line:col:` prefix (ruff/flake8/eslint)
/// or a `E###`/`W###`/`F###` issue code.
pub(crate) fn is_lint_line(line:&str) -> bool {
	let t = line.trim_start();
	is_error_line(t) || is_warning_line(t) || lint_prefix(t) || lint_code(t)
}

/// `path:line:col:` prefix (ruff/flake8/eslint): `splitn(4, ':')` -> a
/// non-empty path without spaces, then two all-digit fields, then a third
/// segment (the trailing `:`). Replaces the `^[^\s:]+:\d+:\d+:` alternative
/// of the old LINT_RE.
fn lint_prefix(t:&str) -> bool {
	let mut it = t.splitn(4, ':');
	let path = match it.next() {
		Some(p) if !p.is_empty() && !p.contains(' ') => p,
		_ => return false,
	};
	let line_no = match it.next() {
		Some(l) if !l.is_empty() && l.chars().all(|c| c.is_ascii_digit()) => l,
		_ => return false,
	};
	let col = match it.next() {
		Some(c) if !c.is_empty() && c.chars().all(|c| c.is_ascii_digit()) => c,
		_ => return false,
	};
	let _ = (path, line_no, col);
	it.next().is_some()
}

/// `E###`/`W###`/`F###` issue code (3-4 digits after the letter) as a
/// whitespace-delimited token. Replaces the `(?:^|\s)[EWF]\d{3,4}\b`
/// alternative of the old LINT_RE.
fn lint_code(t:&str) -> bool {
	t.split_whitespace().any(|tok| {
		let b = tok.as_bytes();
		b.len() >= 4 && b.len() <= 5 && matches!(b[0], b'E' | b'W' | b'F') && b[1..].iter().all(|c| c.is_ascii_digit())
	})
}
