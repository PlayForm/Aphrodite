//! `N passed` / `N failed` count-line predicate.

/// True when a digit immediately precedes `Kw` on the line.
///
/// pytest summaries write `12 passed, 1 failed`, so the presence of a digit
/// run in front of the keyword is the whole test. It is not a count parser:
/// `NumberBefore::Exact::Fn` returns the number, this only answers yes or no,
/// which is all the detector needs to vote.
pub(crate) fn Fn(Line:&str, Kw:&str) -> bool {
	match Line.find(Kw) {
		Some(i) => Line[..i].trim_end().chars().last().map(|c| c.is_ascii_digit()).unwrap_or(false),
		None => false,
	}
}
