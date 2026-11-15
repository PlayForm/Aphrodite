//! `Cstr`: the caller's pointer as an owned `String`.

use std::{ffi::CStr, os::raw::c_char, ptr::NonNull};

/// Convert a caller-supplied C string pointer into an owned `String`.
///
/// A null pointer yields `None`, and each export decides for itself whether
/// that means "argument absent" (substituted with an empty default) or "request
/// invalid" (answered with an error payload). `NonNull::new` is what carries
/// that decision: it turns the nullability into a value before any dereference,
/// so the `unsafe` block below has one reason to exist instead of two.
///
/// It proves nothing about the pointee - `NonNull` is not a validity claim - so
/// the block keeps its contract, and the caller's contract is unchanged.
///
/// # Safety
/// `Ptr`, if non-null, must point to a valid, NUL-terminated C string that
/// stays valid for the duration of this call (the standard `CStr::from_ptr`
/// contract). Every caller in this crate passes pointers received directly
/// from the C ABI call, which are expected to uphold that contract.
pub(crate) unsafe fn Fn(Ptr:*const c_char) -> Option<String> {
	let Ptr = NonNull::new(Ptr.cast_mut())?;

	// SAFETY: `Ptr` is non-null, and per the safety contract of this function
	// it points to a valid, NUL-terminated C string that remains valid for the
	// duration of this call.
	Some(unsafe { CStr::from_ptr(Ptr.as_ptr()) }.to_string_lossy().into_owned())
}
