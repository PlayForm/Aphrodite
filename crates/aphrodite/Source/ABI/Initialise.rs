//! `AphroditeInit` - allocate a session handle, optionally seeded from a
//! configuration file.

use std::os::raw::c_char;

use crate::{
	ABI::{
		Boundary::{Answer::Enum::Enum as Answer, Run::Fn::Fn as Run},
		Guard::Cstr::Fn::Fn as Cstr,
	},
	State::Struct::AphroditeState,
};

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
	// Install the process-wide tracing subscriber before anything can log: the
	// dylib is the plugin's only chance to own the stack, and a host that
	// already installed one is handled inside `Logging::Fn` (it answers `Ok`
	// and keeps the host's). A failure here must not fail the session - a
	// process without a subscriber still compresses; it just says less.
	let _ = crate::Logging::Fn::Fn(false, None);
	let Path = unsafe { Cstr(ConfigurationPath) }.unwrap_or_default();

	Run(move || {
		let mut State = AphroditeState::default();

		// 01-F4/F9: delegate to `ConfigurationLoader::Configuration::Struct::Struct` instead of hand-parsing
		// four `[compression]` keys here - this hand-rolled copy had already
		// drifted from `ApplyCompression`'s own key names (e.g. `ToolThreshold`
		// vs `ToolThresholdToken`) and never honored env var overrides, unlike
		// every other init path in the crate.
		(!Path.is_empty()).then(|| {
			crate::ConfigurationLoader::Configuration::Struct::Struct::LoadFrom(&Path).ApplyCompression(&mut State);
		});

		Ok(Answer::Text(
			crate::ABI::Handle::Registry::Const::Handles.Allocate(State).to_string(),
		))
	})
}
