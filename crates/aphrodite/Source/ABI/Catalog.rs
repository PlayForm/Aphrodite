//! `AphroditeCatalog` - list the session's stored entries as a catalogue.

use std::os::raw::c_char;

use super::{
	Guard::{Cstr, Guarded, ToJSONError, ToJSONOk},
	Handle::GetHandle,
};

/// Answer with the session's catalogue in the requested `Mode`.
///
/// The catalogue is what a caller reads when it remembers the content but not
/// the hash; `Mode` selects the verbosity (`full` when the caller passes
/// something unrecognised). It is a read: nothing is marked retrieved and no
/// teaching signal is recorded, which is exactly why a caller that then expands
/// an entry still has to retrieve it.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeCatalog(Handle:*const c_char, Mode:*const c_char) -> *mut c_char {
	let Hid = match unsafe { Cstr(Handle) }.unwrap_or_default().parse::<usize>() {
		Ok(ID) => ID,
		Err(_) => return ToJSONError("invalid handle"),
	};
	let m = unsafe { Cstr(Mode) }.unwrap_or_default();
	Guarded(std::panic::AssertUnwindSafe(move || {
		match GetHandle(Hid) {
			Some(Session) => {
				let s = Session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
				ToJSONOk(&crate::Catalog::Build::Fn::Fn(&s, &m))
			},
			None => ToJSONError(&format!("invalid handle: {}", Hid)),
		}
	}))
}
