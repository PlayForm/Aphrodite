//! `AphroditeResolve` - expand a hash recursively, markers and all.

use std::os::raw::c_char;

use crate::ABI::{
	Guard::{Cstr, ToJSONError, ToJSONOk},
	Handle::Lookup::WithState,
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
	let Hid = match unsafe { Cstr(Handle) }.unwrap_or_default().parse::<usize>() {
		Ok(ID) => ID,
		Err(_) => return ToJSONError("invalid handle"),
	};
	let h = unsafe { Cstr(hash) }.unwrap_or_default();
	match WithState(Hid, |s| {
		match crate::Resolve::Expand::Fn(s, &h) {
			Some(Content) => serde_json::json!({"found":true,"content":Content}),
			None => serde_json::json!({"found":false}),
		}
	}) {
		Ok(v) => ToJSONOk(&v),
		Err(e) => ToJSONError(&format!("{e:#}")),
	}
}
