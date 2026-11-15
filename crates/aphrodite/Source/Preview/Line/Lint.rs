//! Linter issue-line predicate.
//!
//! ruff, flake8, eslint and clippy all print issues in recognisable shapes. The lint arm needs
//! one predicate that covers them, and it must include genuine compiler errors and warnings
//! because a clippy run interleaves them.

use crate::Preview::Line::{Error, Warning};

/// True for a linter issue line.
///
/// Matches a `path:line:col:` prefix (ruff/flake8/eslint) or an `E###`/`W###`/`F###` issue
/// code, and also any line `Error::Fn` or `Warning::Fn` already accepts. It is not a diagnostic
/// parser: the preview shows the first issue line verbatim, so this only has to find it.
pub(crate) fn Fn(Line:&str) -> bool {
	let Trimmed = Line.trim_start();

	Error::Fn(Trimmed) || Warning::Fn(Trimmed) || LintPrefix(Trimmed) || LintCode(Trimmed)
}

/// `path:line:col:` prefix (ruff/flake8/eslint).
///
/// `splitn(4, ':')` must yield a non-empty path without spaces, then two all-digit fields, then
/// a third segment. Replaces the `^[^\s:]+:\d+:\d+:` alternative of the old lint regex, and is
/// spelled as four `Option` tests so the ordered rejection reads as one predicate rather than as
/// a nest.
fn LintPrefix(Trimmed:&str) -> bool {
	let mut Fields = Trimmed.splitn(4, ':');
	let Path = Fields.next().filter(|Path| !Path.is_empty() && !Path.contains(' '));
	let LineNumber = Fields
		.next()
		.filter(|Field| !Field.is_empty() && Field.chars().all(|c| c.is_ascii_digit()));
	let Column = Fields
		.next()
		.filter(|Field| !Field.is_empty() && Field.chars().all(|c| c.is_ascii_digit()));

	Path.is_some() && LineNumber.is_some() && Column.is_some() && Fields.next().is_some()
}

/// `E###`/`W###`/`F###` issue code as a whitespace-delimited token.
///
/// Three to four digits after the letter. Replaces the `(?:^|\s)[EWF]\d{3,4}\b` alternative of
/// the old lint regex.
fn LintCode(Trimmed:&str) -> bool {
	Trimmed.split_whitespace().any(|Token| {
		let Bytes = Token.as_bytes();

		Bytes.len() >= 4
			&& Bytes.len() <= 5
			&& matches!(Bytes[0], b'E' | b'W' | b'F')
			&& Bytes[1..].iter().all(|Byte| Byte.is_ascii_digit())
	})
}
