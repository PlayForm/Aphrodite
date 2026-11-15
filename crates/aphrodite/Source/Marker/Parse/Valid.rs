//! Hash validity - the gate between "looks like a hash" and "is a hash".
//!
//! Two shapes are valid: a 24+ character hex full hash, or the `i:` inline
//! prefix followed by 6+ hex characters. Both checks are hex-only on purpose -
//! an unanchored "anything that is not a delimiter" class is what previously
//! let a hash capture run across a newline and swallow other markers.

/// Check if a string is a valid CCR hash (>=24 hex chars, or `i:` prefix with
/// >=6 hex chars).
pub fn IsValidCCRHash(h:&str) -> bool {
	if h.len() < 8 {
		return false;
	}
	let h = h.to_lowercase();
	if let Some(Stripped) = h.strip_prefix("i:") {
		Stripped.len() >= 6 && Stripped.chars().all(|c| c.is_ascii_hexdigit())
	} else {
		h.len() >= 24 && h.chars().all(|c| c.is_ascii_hexdigit())
	}
}
