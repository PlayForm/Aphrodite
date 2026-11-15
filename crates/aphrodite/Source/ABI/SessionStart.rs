//! `AphroditeSessionStart` - initialise a session's turn state.

use std::os::raw::c_char;

use crate::ABI::{
	Guard::{Cstr, ToJSONError, ToJSONOk},
	Handle::Lookup::WithState,
};

/// Run the `OnSessionStart` hook body and answer with its JSON result.
///
/// A host calls this once per session, before any tool traffic. It resets every
/// per-session field - the turn counter, the conversation index, the marker
/// list, the referenced-file list, the tool events and the ephemeral directives
/// - and restores the adaptive chain-split threshold to its configured floor.
///
/// That reset is exactly why calling it mid-session is a mistake rather than an
/// error: nothing is corrupted, but the turn view the model has already been
/// shown is discarded. The state is process-global and outlives a session, so a
/// field this hook forgets to clear leaks one conversation into the next.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeSessionStart(Handle:*const c_char) -> *mut c_char {
	let Hid = match unsafe { Cstr(Handle) }.unwrap_or_default().parse::<usize>() {
		Ok(ID) => ID,
		Err(_) => return ToJSONError("invalid handle"),
	};
	match WithState(Hid, crate::Session::Start::Fn::OnSessionStart) {
		Ok(v) => ToJSONOk(&v),
		Err(e) => ToJSONError(&format!("{e:#}")),
	}
}
