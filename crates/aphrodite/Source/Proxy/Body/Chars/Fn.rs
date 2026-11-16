//! The longest prefix of a string that fits a CHARACTER budget, borrowed.
//!
//! The cache-mode marker used to build this prefix into an owned `String`
//! (`chars().take(512).collect()`) and then hand a reference to the temporary,
//! although the formatter only ever needs a `&str` - so the allocation bought
//! nothing and the preview was one more copy of content the caller already
//! holds.
//!
//! The budget is in characters, not bytes, and that is why this is not
//! `Body::Truncate`: a byte budget would cut a CJK preview to a third of the
//! length an ASCII one gets, which changes what the model is shown.

/// The first `Maximum` characters of `s`, borrowed from `s`.
pub(crate) fn Fn(s:&str, Maximum:usize) -> &str {
	match s.char_indices().nth(Maximum) {
		// The byte offset of the first character past the budget is a char
		// boundary by construction, so the slice cannot panic.
		Some((Offset, _)) => &s[..Offset],
		None => s,
	}
}
