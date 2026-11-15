//! `AphroditeInit` - allocate a session handle, optionally seeded from a
//! configuration file.

use std::{ffi::CString, os::raw::c_char};

use super::{
	Guard::{Cstr, Guarded},
	Handle::AllocHandle,
};
use crate::State::AphroditeState;

/// Allocate a session and return its handle as a decimal C string.
///
/// An empty or null `ConfigurationPath` is normal, not an error: the session starts
/// on `AphroditeState::default()` and every later export can still drive it.
/// A path that exists but cannot be read or parsed also degrades to defaults -
/// startup must never be the thing that stops a host from running.
///
/// The handle is the only way back to this session; there is no lookup by
/// configuration or by process, so a caller that loses the string leaks the
/// session until the process exits.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeInit(ConfigurationPath:*const c_char) -> *mut c_char {
	let path = unsafe { Cstr(ConfigurationPath) }.unwrap_or_default();
	Guarded(std::panic::AssertUnwindSafe(move || {
		let mut State = AphroditeState::default();
		// 01-F4/F9: delegate to `ConfigurationLoader::Configuration::Struct::Struct` instead of hand-parsing
		// four `[compression]` keys here - this hand-rolled copy had already
		// drifted from `ApplyCompression`'s own key names (e.g. `ToolThreshold`
		// vs `ToolThresholdToken`) and never honored env var overrides, unlike
		// every other init path in the crate.
		if !path.is_empty() {
			crate::ConfigurationLoader::Configuration::Struct::Struct::LoadFrom(&path).ApplyCompression(&mut State);
		}
		CString::new(AllocHandle(State).to_string()).unwrap().into_raw()
	}))
}
