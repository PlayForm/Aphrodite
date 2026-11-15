//! `AphroditeDestroy` - release a session handle.

use std::os::raw::c_char;

use crate::ABI::{Guard::Cstr, Handle::Table::Handles};

/// Drop the session named by `Handle` from the handle table.
///
/// An unparseable or unknown handle is a no-op rather than an error: destroy is
/// the cleanup path, and a caller tidying up a session it never successfully
/// created must not have to branch. The state itself is freed when the last
/// `Arc` reference drops, so an export still running against this session
/// finishes safely.
///
/// After this returns, every stateful call on the handle reports
/// "invalid handle" - it never revives the session.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeDestroy(Handle:*const c_char) {
	if let Ok(Hid) = unsafe { Cstr(Handle) }.unwrap_or_default().parse::<usize>() {
		Handles().as_mut().and_then(|m| m.remove(&Hid));
	}
}
