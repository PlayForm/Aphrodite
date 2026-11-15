//! `aphrodite_hermes_version` - this crate's version.

use std::os::raw::c_char;

use crate::Guard::Fn::{guarded, to_c_string};

/// Version of this crate.
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_version() -> *mut c_char {
	guarded(|| to_c_string(&serde_json::json!({"version": env!("CARGO_PKG_VERSION")}).to_string()))
}
