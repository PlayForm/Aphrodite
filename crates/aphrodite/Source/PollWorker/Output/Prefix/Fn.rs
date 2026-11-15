//! The longest char-aligned prefix of a payload that fits in the room left.

/// The prefix of `Content` that fits in `Room` bytes, or `None` when not even one
/// character fits.
///
/// Cutting on a `char` boundary is what keeps the accumulator valid UTF-8: a
/// byte slice at an arbitrary offset would panic on the next `push_str` of a
/// borrowed slice and would render as a replacement character to the model.
pub(crate) fn Fn(Content:&str, Room:usize) -> Option<&str> {
	let End = Content
		.char_indices()
		.map(|(Index, Char)| Index + Char.len_utf8())
		.take_while(|End| *End <= Room)
		.last()?;

	Some(&Content[..End])
}
