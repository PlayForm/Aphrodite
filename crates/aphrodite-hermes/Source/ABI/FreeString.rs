//! `aphrodite_hermes_free_string` - reclaim a string an export returned.
//!
//! The ONE export that is not panic-guarded, deliberately: it must stay
//! minimal and infallible, because it is the only way the caller can release a
//! returned string and a guard here would have to allocate on the failure path.
//! It pairs with `Guard::to_c_string`'s `CString::into_raw` - freeing that
//! allocation any other way is an invalid free. A null pointer is a no-op so a
//! caller can free unconditionally.

use std::{ffi::CString, os::raw::c_char};

/// Free a string returned by any aphrodite_hermes_* function.
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_free_string(s:*mut c_char) {
	match s.is_null() {
		true => {},
		false => unsafe {
			let _ = CString::from_raw(s);
		},
	}
}
