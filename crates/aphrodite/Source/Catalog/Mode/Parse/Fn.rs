//! Read the caller's mode.

use super::super::Enum;

/// Parse the wire `Mode` string.
///
/// One comparison, at the boundary, so the rest of the catalog matches on a value.
/// Anything that is not exactly `"toc"` is `Full`: that is what the builder's
/// `Mode == "toc"` tests meant, and a caller passing an unrecognised mode must keep
/// getting the complete projection rather than an empty one.
pub fn Fn(Mode:&str) -> Enum::Enum {
	match Mode {
		"toc" => Enum::Enum::TOC,
		_ => Enum::Enum::Full,
	}
}
