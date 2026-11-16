//! `Handle`: the caller's handle pointer as a session id.

use std::os::raw::c_char;

use crate::ABI::Guard::Cstr::Enum::Enum as Decoded;

/// Decode the caller's handle argument into a session id.
///
/// `None` covers the three shapes that all answer `invalid handle`, and this is
/// the one place where collapsing them is right rather than lazy: a null pointer,
/// a string that is not a decimal `usize`, and a string past the ingress cap all
/// say the same thing to the caller - the handle it holds does not name a live
/// session - and it has no different action to take for any of them.
///
/// The cap is checked here too because a handle arrives through the same ingress
/// as every other argument; a 16 MB "handle" is refused before it is parsed, not
/// after.
///
/// # Safety
/// `Ptr`, if non-null, must point to a valid, NUL-terminated C string that stays
/// readable and unmoved for the duration of this call.
pub(crate) unsafe fn Fn(Ptr:*const c_char) -> Option<usize> {
	match unsafe { crate::ABI::Guard::Cstr::Fn::Fn(Ptr) } {
		Decoded::Text(Text) => Text.parse().ok(),
		Decoded::Absent | Decoded::Oversize => None,
	}
}
