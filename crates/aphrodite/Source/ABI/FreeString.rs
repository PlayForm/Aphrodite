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
	if !s.is_null() {
		unsafe {
			let _ = CString::from_raw(s);
		}
	}
}
