//! The quoting state a command scanner is in at one character.

/// Which quote, if any, encloses the character under the scanner.
///
/// It is a value rather than a pair of booleans because the two booleans admit
/// a state the shell cannot: both set at once. A separator only splits the
/// chain in `None`, and a quote character only toggles when it is the quote
/// that opened the run - `'` inside `"…"` is literal text, not a delimiter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enum {
	/// Outside every quote: separators split, quotes open.
	None,
	/// Inside `'…'`: only `'` is significant.
	Single,
	/// Inside `"…"`: only `"` and `\` are significant.
	Double,
}
