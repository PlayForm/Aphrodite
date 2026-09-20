//! Signature formatting and byte-safe truncation helpers.

use super::{MAX_PARAMS_LEN, MAX_SIG_LEN};

/// Byte-safe prefix of `s` no longer than `max` bytes - never splits a
/// multi-byte UTF-8 character, unlike a raw `&s[..max]` (which panics if
/// `max` falls inside a codepoint; source content is arbitrary UTF-8, not
/// guaranteed ASCII-aligned at any fixed offset).
pub(crate) fn floor_boundary(s:&str, max:usize) -> &str {
	if s.len() <= max {
		return s;
	}
	let mut i = max;
	while !s.is_char_boundary(i) {
		i -= 1;
	}
	&s[..i]
}

/// Truncate a signature to fit the preview budget.
pub(crate) fn sig(kind:&str, text:&str) -> String {
	let s = format!("{} {}", kind, text).trim().to_string();
	if s.len() > MAX_SIG_LEN {
		floor_boundary(&s, MAX_SIG_LEN - 3).to_string() + "..."
	} else {
		s
	}
}

pub(crate) fn trunc_params(params:&str) -> String {
	if params.len() > MAX_PARAMS_LEN {
		floor_boundary(params, MAX_PARAMS_LEN - 3).to_string() + "..."
	} else {
		params.to_string()
	}
}
