//! `AphroditeFreeString` - the single deallocator for every pointer this
//! ABI hands out.

use std::{ffi::CString, os::raw::c_char};

/// Take back a string this ABI allocated and drop it.
///
/// Every pointer-returning export transfers ownership here. Freeing such a
/// pointer with the C runtime's `free` (or Python's `ctypes` default) is the
/// historical allocator-mismatch abort: the Rust allocator owns the block, so
/// only Rust may release it.
///
/// A null pointer is a no-op, which makes the caller's cleanup path
/// unconditional.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeFreeString(s:*mut c_char) {
	// `NonNull` turns "the caller may pass null" into a value, so the `unsafe`
	// block below has one reason to exist and one contract to state.
	let Some(s) = std::ptr::NonNull::new(s) else { return };

	// SAFETY: `s` was allocated by `CString::into_raw` inside this ABI, and this
	// is the only place that reclaims it - a caller that frees the same pointer
	// twice, or with `free`, has already broken the contract this function
	// documents. The null case never reaches here.
	let _ = unsafe { CString::from_raw(s.as_ptr()) };
}
