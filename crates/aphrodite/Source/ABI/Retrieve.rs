//! `AphroditeRetrieve` - hand back the stored payload behind a hash.

use std::{ffi::CString, os::raw::c_char};

use crate::ABI::{
	Guard::{Cstr, Guarded, ToJSONError},
	Handle::Lookup::GetHandle,
};

/// Return the raw content stored under `hash` - not a JSON envelope.
///
/// This is the one stateful export whose success payload is the content itself,
/// because the caller's next move is to put it back into context verbatim; a
/// JSON wrapper would have to be unescaped by every host. Failures are still
/// `{"error": ...}` JSON, so a caller must distinguish "content that happens to
/// start with a brace" from an error by checking for the `error` key, not by
/// shape alone.
///
/// Retrieval is also the teaching loop: the FFI path bypasses `ResolveOne`, so
/// the consequence is attributed here instead, which is what lets the engine
/// learn which markers are actually worth keeping.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeRetrieve(Handle:*const c_char, hash:*const c_char) -> *mut c_char {
	let Hid = match unsafe { Cstr(Handle) }.unwrap_or_default().parse::<usize>() {
		Ok(ID) => ID,
		Err(_) => return ToJSONError("invalid handle"),
	};
	let hash = unsafe { Cstr(hash) }.unwrap_or_default();
	Guarded(std::panic::AssertUnwindSafe(move || {
		let Session = match GetHandle(Hid) {
			Some(s) => s,
			None => return ToJSONError(&format!("invalid handle: {}", Hid)),
		};
		let mut s = Session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
		match s.InlineStoreGet(&hash) {
			Some(Content) => {
				// Tier 1 teaching loop: the FFI retrieve path bypasses
				// resolve_one, so attribute the consequence here too.
				s.NoteSplitRetrieval(&hash);
				CString::new(Content.replace('\0', "")).unwrap().into_raw()
			},
			None => ToJSONError(&format!("hash not found: {}", hash)),
		}
	}))
}
