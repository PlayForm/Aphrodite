//! Classify one diff line.

use crate::Stage2::Reduce::Diff::Event::Enum::Enum;

/// Classify a line as a file header, a hunk header, or neither.
///
/// The two prefixes are the ones the previous reducer tested. They are mutually exclusive, so the
/// order of the two tests cannot change the answer - and because a line is classified once, the
/// reducer never re-tests it. It is not a diff parser: a bare unified diff with no `diff --git`
/// header is entirely `Other`, which is why such content reduces to nothing.
pub(crate) fn Fn(Line:&str) -> Enum<'_> {
	Line.starts_with("@@ ")
		.then_some(Enum::Hunk)
		.or_else(|| Line.starts_with("diff --git ").then_some(Enum::File { Header:Line }))
		.unwrap_or(Enum::Other)
}
