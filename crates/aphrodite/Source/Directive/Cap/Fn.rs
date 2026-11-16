//! The per-file directive size cap.

use std::borrow::Cow;

use super::Single::Const::Const as Maximum;

/// Cut a directive body to the per-file cap, appending `...` when it was cut.
///
/// Borrowed when the body fits, which is the normal case: a cap that copied every
/// directive would allocate once per file to hand back the same bytes.
///
/// The cap is found by asking for the character AFTER the last one the cap allows.
/// `char_indices().nth(Maximum)` answers the byte offset of that character, and its
/// presence IS the "content is over the cap" test - so the comparison and the cut
/// boundary cannot disagree, and no multibyte character can be split. The previous
/// shape compared `Content.len()` (bytes) and then cut by characters, which cut
/// content that fit the character budget.
pub(crate) fn Fn(Content:&str) -> Cow<'_, str> {
	match Content.char_indices().nth(Maximum) {
		Some((Offset, _Char)) => Cow::Owned(format!("{}...", &Content[..Offset])),
		None => Cow::Borrowed(Content),
	}
}
