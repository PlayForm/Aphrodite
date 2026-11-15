//! C string marshalling and the panic guard for the `extern "C"` boundary.

use std::{
	ffi::{CStr, CString},
	os::raw::c_char,
};

/// Convert a caller-supplied C string pointer into an owned `String`, or an
/// empty string if `ptr` is null. Invalid UTF-8 is replaced lossily rather
/// than rejected, since a malformed argument should degrade gracefully, not
/// panic across the `extern "C"` boundary.
///
/// # Safety
/// `ptr`, if non-null, must point to a valid, NUL-terminated C string that
/// stays valid for the duration of this call (the standard `CStr::from_ptr`
/// contract). Every caller in this crate passes pointers received directly
/// from Hermes's C ABI call, which are expected to uphold that contract.
pub(crate) unsafe fn cstr_to_string(ptr:*const c_char) -> String {
	match ptr.is_null() {
		true => String::new(),
		false => {
			// SAFETY: `ptr` is non-null here, and per the safety contract of this
			// function it points to a valid, NUL-terminated C string that stays
			// valid for the duration of this call.
			unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned()
		},
	}
}

/// Hand a Rust string to the caller as an owned C string.
///
/// The allocation is reclaimed ONLY by `aphrodite_hermes_free_string`
/// (`CString::from_raw`); freeing it any other way - `libc::free`, a
/// different allocator - is an invalid free. An interior NUL makes the
/// conversion fail, which yields a null pointer rather than a truncated
/// string: the caller sees "no result", never a silently mangled one.
pub(crate) fn to_c_string(s:&str) -> *mut c_char {
	CString::new(s).map(|c| c.into_raw()).unwrap_or(std::ptr::null_mut())
}

/// Encode a failure as `{"error": "..."}` for the caller.
///
/// Every export's error path goes through here so the Python shim can branch
/// on one shape instead of guessing from free text.
pub(crate) fn to_json_error(msg:&str) -> *mut c_char { to_c_string(&serde_json::json!({"error": msg}).to_string()) }

/// Run `f` under `catch_unwind`, converting a panic into an error-JSON string
/// instead of letting it unwind across the `extern "C"` boundary (which would
/// otherwise trigger the Rust runtime's forced process abort - this crate had
/// zero panic guards until this was added). Every exported fn except
/// `aphrodite_hermes_free_string` (must stay minimal/infallible) routes
/// through this.
pub(crate) fn guarded(f:impl FnOnce() -> *mut c_char + std::panic::UnwindSafe) -> *mut c_char {
	std::panic::catch_unwind(f).unwrap_or_else(|_| to_json_error("internal error: panicked in aphrodite-hermes"))
}
