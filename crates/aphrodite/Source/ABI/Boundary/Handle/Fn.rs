//! `Handle`: the caller's handle pointer as a session id.

use std::os::raw::c_char;

/// Decode the caller's handle argument into a session id.
///
/// `None` covers both shapes the ABI has always collapsed into one answer: a
/// null pointer (no handle at all) and a string that is not a decimal `usize`.
/// Both answer `invalid handle`, because a host cannot act differently on the
/// two - the handle it holds either names a live session or it does not.
///
/// # Safety
/// `Ptr`, if non-null, must point to a valid, NUL-terminated C string that
/// stays valid for the duration of this call.
pub(crate) unsafe fn Fn(Ptr:*const c_char) -> Option<usize> {
	unsafe { crate::ABI::Guard::Cstr::Fn::Fn(Ptr) }.unwrap_or_default().parse().ok()
}
