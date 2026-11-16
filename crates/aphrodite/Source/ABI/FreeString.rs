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
///
/// # Safety
/// `Pointer` must be either null or the exact pointer this ABI returned, still
/// unmodified: same allocation, same starting address, same terminator position.
/// `CString::from_raw` recomputes the allocation's length from the terminator, so
/// writing an earlier NUL into the buffer or removing the final one makes the
/// reconstruction describe a different block than the one that was allocated.
///
/// The two rules that follow are the whole contract: treat a returned string as
/// read-only, and hand the pointer back to this function exactly once - never to
/// `free`, never to a second call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AphroditeFreeString(Pointer:*mut c_char) {
	// `NonNull` turns "the caller may pass null" into a value, so the `unsafe`
	// block below has one reason to exist and one contract to state.
	let Some(Pointer) = std::ptr::NonNull::new(Pointer) else { return };

	// SAFETY: `Pointer` was allocated by `CString::into_raw` inside this ABI, and
	// this is the only place that reclaims it - a caller that frees the same
	// pointer twice, or with `free`, has already broken the contract this
	// function documents. The null case never reaches here.
	let _ = unsafe { CString::from_raw(Pointer.as_ptr()) };
}
