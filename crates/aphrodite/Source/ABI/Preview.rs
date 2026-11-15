//! `AphroditePreview` - build the honest preview string for content.

use std::{ffi::CString, os::raw::c_char};

use super::Guard::{Cstr, Guarded, ToJSONError};

/// Generate preview for content - port of _marker/preview.py
///
/// Stateless, and the same builder the compression path uses, which is the
/// point: a caller that shows one preview and stores another has told the model
/// something the store does not contain. The answer is the preview text itself.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditePreview(Content:*const c_char, CCRType:*const c_char) -> *mut c_char {
	let c = match unsafe { Cstr(Content) } {
		Some(s) => s,
		None => return ToJSONError("null content"),
	};
	let t = unsafe { Cstr(CCRType) }.unwrap_or_default();
	Guarded(std::panic::AssertUnwindSafe(move || {
		let Preview = crate::BuildPreview(&t, &c);
		CString::new(Preview.replace('\0', "")).unwrap().into_raw()
	}))
}
