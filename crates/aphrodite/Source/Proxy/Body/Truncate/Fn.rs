//! Truncate to a byte budget without splitting a UTF-8 character.
//!
//! `&s[..max]` panics when `Maximum` lands inside a multibyte char - reachable from
//! a non-ASCII header value in the dev-mode request log. Walk back to the last
//! char boundary instead (at most 3 bytes for UTF-8).

/// Truncate a string to at most `Maximum` bytes without splitting a UTF-8
/// char. `&s[..max]` panics (`is_char_boundary`) when `Maximum` lands inside a
/// multibyte char - e.g. non-ASCII header values in the dev-mode request
/// log. Walk back to the last char boundary instead (max 3 bytes for
/// UTF-8).
pub(crate) fn Fn(s:&str, Maximum:usize) -> &str {
	match s.len() <= Maximum {
		true => s,
		false => &s[..(0..=Maximum).rev().find(|&b| s.is_char_boundary(b)).unwrap_or(0)],
	}
}
