//! `AphroditeVersion` - the engine's own version string.

use std::{ffi::CString, os::raw::c_char};

/// Report the version of the linked engine as a bare C string.
///
/// The value is `CARGO_PKG_VERSION` baked in at compile time, not the runtime
/// `BINARY_VERSION` pin the plugin keeps: a caller comparing the two is asking
/// whether the dylib on disk matches the pointer, which is a plugin-side
/// question, not one this function can answer.
///
/// The string is heap-allocated and ownership transfers to the caller - release
/// it with `AphroditeFreeString`, never with `free`.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeVersion() -> *mut c_char { CString::new(env!("CARGO_PKG_VERSION")).unwrap().into_raw() }
