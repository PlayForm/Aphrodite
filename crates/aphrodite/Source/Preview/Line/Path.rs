//! Bare path-like token predicate.
//!
//! `find` and plain `ls` print one path per line with nothing else on it. The directory arm
//! counts those lines, so it needs to know that a bare token is a path - and must not count a
//! bare word like `Total` or `README` as one.

/// True when a line is a bare path-like token.
///
/// A single whitespace-free token that contains a `/` or has an extension. It is not a
/// filesystem check: nothing is stat'ed, so a path-shaped token that does not exist still
/// counts. That is intentional - the preview describes the text the tool produced, not the disk.
pub(crate) fn Fn(Line:&str) -> bool {
	let Trimmed = Line.trim();
	let Bare = !Trimmed.is_empty() && !Trimmed.contains(char::is_whitespace);

	Bare && (Trimmed.contains('/') || Trimmed.rfind('.').is_some_and(|Dot| Dot > 0 && Dot < Trimmed.len() - 1))
}
