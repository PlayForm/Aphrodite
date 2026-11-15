//! `running N tests` header predicate.

/// True for a `running N tests` header line.
///
/// A bare test log can arrive without a summary line, and the header is then
/// the only evidence that the content is test output. The digit run is
/// required, `test`/`Tests` must end at a word boundary, and anything else
/// fails. It is not a summary reader - it only recognises the header.
pub(crate) fn Fn(t:&str) -> bool {
	let Rest = match t.strip_prefix("running ") {
		Some(r) => r,
		None => return false,
	};
	let Digits = Rest.chars().take_while(|c| c.is_ascii_digit()).count();
	if Digits == 0 {
		return false;
	}
	let After = Rest[Digits..].trim_start();
	let After = match After.strip_prefix("test") {
		Some(a) => a,
		None => return false,
	};
	let After = match After.strip_prefix('s') {
		Some(a) => a,
		None => After,
	};
	// `tests?\b`: the word must end here or be followed by a non-word char.
	After.chars().next().map(|c| !(c.is_alphanumeric() || c == '_')).unwrap_or(true)
}
