//! git-log shape detector.
//!
//! A log is a run of `commit <hash>` blocks. One block could be a quoted line of
//! prose, so the detector needs corroboration. It is not a history reader: the
//! commits are never resolved.

use crate::Preview::Input::Struct::Struct as Input;

/// True when the content looks like `git log` output.
///
/// At least one `commit <hash>` block, and either two blocks or an `Author:`
/// line. A lone `commit ` line in prose is not a log. It is not a range check:
/// no parent links or branches are examined.
pub(crate) fn Fn(Inp:&Input<'_>) -> bool {
	let CommitLines = Inp
		.NonEmpty
		.iter()
		.filter(|l| {
			l.strip_prefix("commit ")
				.map(|h| h.trim().len() >= 7 && h.trim().chars().take(7).all(|c| c.is_ascii_hexdigit()))
				.unwrap_or(false)
		})
		.count();
	CommitLines >= 1 && (CommitLines >= 2 || Inp.raw.contains("Author:"))
}
