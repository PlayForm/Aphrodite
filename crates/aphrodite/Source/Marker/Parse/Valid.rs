//! Hash validity - the gate between "looks like a hash" and "is a hash".
//!
//! Two shapes are valid: a 24+ character hex full hash, or the `i:` inline
//! prefix followed by 6+ hex characters. Both checks are hex-only on purpose -
//! an unanchored "anything that is not a delimiter" class is what previously
//! let a hash capture run across a newline and swallow other markers.

/// Check if a string is a valid CCR hash (>=24 hex chars, or `i:` prefix with
/// >=6 hex chars).
///
/// External contract: `Hash` is a candidate taken from a marker or from a
/// caller, and `false` means "do not use this as a key" - an empty or short
/// value is simply invalid, never a special case.
///
/// No lowercasing: `is_ascii_hexdigit` accepts both cases, so the copy the
/// previous `to_lowercase()` made on every call bought nothing. The two spellings
/// of the inline prefix are named explicitly instead of being a side effect of
/// that copy - `I:` is accepted because lowercasing used to accept it, and it is
/// written down here so the accepted set is visible rather than incidental.
///
/// There is no length pre-gate: an `i:`/`I:` prefix leaves at most `len - 2`
/// hex characters, so anything under 8 characters already fails the 6-character
/// floor, and the bare-hash arm already demands 24.
pub fn Fn(Hash:&str) -> bool {
	match Hash.strip_prefix("i:").or_else(|| Hash.strip_prefix("I:")) {
		Some(Stripped) => Stripped.len() >= 6 && Stripped.chars().all(|Character| Character.is_ascii_hexdigit()),
		None => Hash.len() >= 24 && Hash.chars().all(|Character| Character.is_ascii_hexdigit()),
	}
}
