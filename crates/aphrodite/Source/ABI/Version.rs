//! `AphroditeVersion` - the engine's own version string.

use std::os::raw::c_char;

use crate::ABI::Boundary::{Answer::Enum::Enum as Answer, Transfer::Fn::Fn as Transfer};

/// Report the version of the linked engine as a bare C string.
///
/// The value is `CARGO_PKG_VERSION` baked in at compile time, not the runtime
/// `BINARY_VERSION` pin the plugin keeps: a caller comparing the two is asking
/// whether the dylib on disk matches the pointer, which is a plugin-side
/// question, not one this function can answer.
///
/// The string is heap-allocated and ownership transfers to the caller - release
/// it with `AphroditeFreeString`, never with `free`.
///
/// Takes no pointer, so it stays a safe function; the allocation still goes
/// through `Boundary::Transfer`, so there is one place in this ABI that mints a
/// pointer and one place that reclaims it.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeVersion() -> *mut c_char { Transfer(Answer::Text(env!("CARGO_PKG_VERSION").to_string())) }
