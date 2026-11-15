//! Cap an error hint at the width a preview can carry.

use std::borrow::Cow;

/// The first 60 CHARACTERS of a hint line, borrowed when the line already fits.
///
/// Borrowed-or-owned because the cap is an optional transform: most hint lines
/// are shorter than the cap, and returning a fresh `String` for them would
/// allocate on every segment of every chained command for no change. The cut
/// is on a char boundary (`char_indices`), never a byte offset, so a multi-byte
/// character is never split.
pub(crate) fn Fn(Line:&str) -> Cow<'_, str> {
	match Line.char_indices().nth(60) {
		Some((Cut, _)) => Cow::Owned(Line[..Cut].to_string()),
		None => Cow::Borrowed(Line),
	}
}
