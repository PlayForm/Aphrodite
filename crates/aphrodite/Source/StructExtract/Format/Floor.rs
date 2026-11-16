//! Byte-safe prefix of a string.

use std::num::NonZeroUsize;

/// The longest prefix of `s` that is at most `Maximum` bytes and ends on a character boundary.
///
/// A raw `&s[..Maximum]` panics when `Maximum` falls inside a multi-byte codepoint, and source content
/// is arbitrary UTF-8: a signature cut at byte 57 of a file full of `e`-with-an-accent is a panic in the
/// preview path, not a cosmetic truncation. It is not a word- or grapheme-aware cut - it walks back to
/// the nearest `is_char_boundary`, so a combining mark can be separated from its base character. The
/// result is always at most `Maximum` bytes and always a valid `&str`, and `s` itself when it already
/// fits - where "fits" includes `s.len() == Maximum`, so a string exactly at the cap is returned whole
/// rather than walked and re-sliced to itself.
///
/// `str::floor_char_boundary` is the standard library's version of this walk and it is NOT used here:
/// it is `#[stable(since = "1.91.0")]` and this workspace declares `rust-version = "1.88"`, so adopting
/// it would raise the crate's MSRV. The walk below is the same search with the same result.
pub(crate) fn Fn(s:&str, Maximum:usize) -> &str {
	match NonZeroUsize::new(s.len().saturating_sub(Maximum)) {
		// Fits already: the whole string, untouched.
		None => s,
		// Over budget: index 0 is always a boundary, so the search always terminates.
		Some(_) => &s[..(0..=Maximum).rev().find(|&Boundary| s.is_char_boundary(Boundary)).unwrap_or(0)],
	}
}
