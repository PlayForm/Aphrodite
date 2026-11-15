//! Linter issue-line predicate.
//!
//! ruff, flake8, eslint and clippy all print issues in recognisable shapes.
//! The lint arm needs one predicate that covers them, and it must include
//! genuine compiler errors and warnings because a clippy run interleaves them.

use crate::Preview::Line::{Error, Warning};

/// True for a linter issue line.
///
/// Matches a `path:line:col:` prefix (ruff/flake8/eslint) or an
/// `E###`/`W###`/`F###` issue code, and also any line `Error::Fn` or
/// `Warning::Fn` already accepts. It is not a diagnostic parser: the preview
/// shows the first issue line verbatim, so this only has to find it.
pub(crate) fn Fn(Line:&str) -> bool {
	let t = Line.trim_start();
	Error::Fn(t) || Warning::Fn(t) || LintPrefix(t) || LintCode(t)
}

/// `path:line:col:` prefix (ruff/flake8/eslint).
///
/// `splitn(4, ':')` must yield a non-empty path without spaces, then two
/// all-digit fields, then a third segment. Replaces the `^[^\s:]+:\d+:\d+:`
/// alternative of the old LINT_RE.
fn LintPrefix(t:&str) -> bool {
	let mut It = t.splitn(4, ':');
	let path = match It.next() {
		Some(p) if !p.is_empty() && !p.contains(' ') => p,
		_ => return false,
	};
	let LineNo = match It.next() {
		Some(l) if !l.is_empty() && l.chars().all(|c| c.is_ascii_digit()) => l,
		_ => return false,
	};
	let Col = match It.next() {
		Some(c) if !c.is_empty() && c.chars().all(|c| c.is_ascii_digit()) => c,
		_ => return false,
	};
	let _ = (path, LineNo, Col);
	It.next().is_some()
}

/// `E###`/`W###`/`F###` issue code as a whitespace-delimited token.
///
/// Three to four digits after the letter. Replaces the `(?:^|\s)[EWF]\d{3,4}\b`
/// alternative of the old LINT_RE.
fn LintCode(t:&str) -> bool {
	t.split_whitespace().any(|Tok| {
		let b = Tok.as_bytes();
		b.len() >= 4 && b.len() <= 5 && matches!(b[0], b'E' | b'W' | b'F') && b[1..].iter().all(|c| c.is_ascii_digit())
	})
}
