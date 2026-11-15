//! `running N tests` header predicate.

/// True for a `running N tests` header line.
///
/// A bare test log can arrive without a summary line, and the header is then the only evidence that the
/// content is test output. The digit run is required, `test`/`Tests` must end at a word boundary, and
/// anything else fails. It is not a summary reader - it only recognises the header.
///
/// The digit run is measured in characters and used as a byte offset, so the tail is taken through
/// `str::get`: the digits are ASCII so the offset is always a boundary, and `get` keeps the reader from
/// depending on that.
pub(crate) fn Fn(Trimmed:&str) -> bool {
	let Rest = Trimmed.strip_prefix("running ").unwrap_or("");
	let Digits = Rest.chars().take_while(|Character| Character.is_ascii_digit()).count();
	let After = match Rest
		.get(Digits..)
		.map(str::trim_start)
		.and_then(|Rest| Rest.strip_prefix("test"))
	{
		Some(After) => After,
		None => return false,
	};
	// `tests?\b`: the optional plural, then the word must end here or be followed by a non-word char.
	let After = After.strip_prefix('s').unwrap_or(After);

	Digits > 0
		&& After
			.chars()
			.next()
			.map(|Character| !(Character.is_alphanumeric() || Character == '_'))
			.unwrap_or(true)
}
