//! Byte-safe prefix of a string.

/// The longest prefix of `s` that is at most `Maximum` bytes and ends on a character
/// boundary.
///
/// A raw `&s[..max]` panics when `Maximum` falls inside a multi-byte codepoint, and
/// source content is arbitrary UTF-8: a signature cut at byte 57 of a file full of
/// `é` is a panic in the preview path, not a cosmetic truncation. It is not a
/// word- or grapheme-aware cut - it walks back to the nearest `is_char_boundary`,
/// so a combining mark can be separated from its base character. The result is
/// always at most `Maximum` bytes and always a valid `&str`, and `s` itself when it
/// already fits.
pub(crate) fn Fn(s:&str, Maximum:usize) -> &str {
	match s.len() <= Maximum {
		true => {
			return s;
		},
		false => {},
	}
	// The walk back to the nearest boundary: index 0 is always a boundary, so the
	// search always terminates.
	let i = (0..=Maximum).rev().find(|&Boundary| s.is_char_boundary(Boundary)).unwrap_or(0);
	&s[..i]
}
