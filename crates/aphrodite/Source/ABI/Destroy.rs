//! `AphroditeDestroy` - release a session handle.

use std::os::raw::c_char;

use crate::ABI::{Boundary::Handle::Fn::Fn as HandleID, Handle::Registry::Const::Handles};

/// Drop the session named by `Handle` from the handle table.
///
/// An unparseable or unknown handle is a no-op rather than an error: destroy is
/// the cleanup path, and a caller tidying up a session it never successfully
/// created must not have to branch. The state itself is freed when the last
/// `Arc` reference drops, so an export still running against this session
/// finishes safely - removal prevents future lookups, it does not cancel an
/// in-flight call.
///
/// After this returns, every stateful call on the handle reports
/// "invalid handle" - it never revives the session.
///
/// This export returns nothing, so it has no envelope to route through the
/// boundary and no panic to intercept: the two operations it performs are a
/// pointer read and a map removal, neither of which can unwind.
/// # Safety
/// `Handle` must be null or point to a valid, NUL-terminated C string that stays
/// readable and unmoved for the duration of this call. Nothing is returned, so
/// there is no allocation to release.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AphroditeDestroy(Handle:*const c_char) {
	let _ = unsafe { HandleID(Handle) }.map(|Identity| Handles.Remove(Identity));
}
