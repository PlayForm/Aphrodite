//! `AphroditeResolve` - expand a hash recursively, markers and all.

use std::os::raw::c_char;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, State::Fn::Fn as State},
	Guard::Cstr::Fn::Fn as Cstr,
};

/// Resolve hash with full recursive expansion - port of _resolve.py
///
/// Answers `{"found":true,"content":...}` or `{"found":false}`. The boolean is
/// part of the contract, not decoration: `found:false` is the only statement
/// that the hash is absent, and a caller must not read an empty `Content` as
/// absence.
///
/// Recursive expansion is what makes this different from `AphroditeRetrieve`:
/// a stored payload that itself contains markers comes back fully expanded, so
/// the result may be far larger than the entry that was asked for.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeResolve(Handle:*const c_char, hash:*const c_char) -> *mut c_char {
	let hash = unsafe { Cstr(hash) }.unwrap_or_default();

	State(Handle, move |s| {
		Ok(Answer::Envelope(crate::Resolve::Expand::Fn(s, &hash).map_or_else(
			|| serde_json::json!({"found":false}),
			|Content| serde_json::json!({"found":true,"content":Content}),
		)))
	})
}
