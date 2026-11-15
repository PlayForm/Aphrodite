//! `AphroditeStage2` - the semantic reduction pass over a payload.

use std::{ffi::CString, os::raw::c_char};

use super::Guard::{Cstr, Guarded, ToJSONError};

/// Stage 2 semantic reduction - port of _stage2.py
///
/// Returns the reduced text, or `{"error":"no reduction possible"}` when the
/// content has no shape this reducer understands. "No reduction possible" is a
/// normal answer, not a failure of the caller: stage 2 is opportunistic, and a
/// caller that treats it as an error will retry content that will never reduce.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeStage2(Content:*const c_char, CCRType:*const c_char) -> *mut c_char {
	let c = match unsafe { Cstr(Content) } {
		Some(s) => s,
		None => return ToJSONError("null content"),
	};
	let t = unsafe { Cstr(CCRType) }.unwrap_or_default();
	Guarded(std::panic::AssertUnwindSafe(move || {
		match crate::Stage2::CompressStage2(&c, &t) {
			Some(Reduced) => CString::new(Reduced.replace('\0', "")).unwrap().into_raw(),
			None => ToJSONError("no reduction possible"),
		}
	}))
}
