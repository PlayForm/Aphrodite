//! `AphroditeInit` - allocate a session handle, optionally seeded from a
//! configuration file.

use std::os::raw::c_char;

use crate::{
	ABI::{
		Boundary::{Answer::Enum::Enum as Answer, Run::Fn::Fn as Run},
		Guard::Cstr::{Fn::Fn as Cstr, Optional::Fn::Fn as Optional},
	},
	State::Struct::AphroditeState,
};

/// Allocate a session and return its handle as a decimal C string.
///
/// An empty or null `Path` is normal, not an error: the session starts
/// on `AphroditeState::default()` and every later export can still drive it.
/// A path that exists but cannot be read or parsed also degrades to defaults -
/// startup must never be the thing that stops a host from running.
///
/// The handle is the only way back to this session; there is no lookup by
/// configuration or by process, so a caller that loses the string leaks the
/// session until the process exits.
/// # Safety
/// `Path` must be null or point to a valid, NUL-terminated C string
/// that stays readable and unmoved for the duration of this call. The returned
/// pointer is owned by the caller and must be released with
/// `AphroditeFreeString`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AphroditeInit(Path:*const c_char) -> *mut c_char {
	// Install the process-wide tracing subscriber before anything can log: the
	// dylib is the plugin's only chance to own the stack, and a host that
	// already installed one is handled inside `Logging::Fn` (it answers `Ok`
	// and keeps the host's). A failure here must not fail the session - a
	// process without a subscriber still compresses; it just says less.
	let _ = crate::Logging::Fn::Fn(false, None);
	let Path = unsafe { Cstr(Path) };

	Run(move || {
		let Path = Optional(Path)?;

		let mut State = AphroditeState::default();

		// 01-F4/F9: delegate to `ConfigurationLoader::Configuration::Struct::Struct` instead of hand-parsing
		// four `[compression]` keys here - this hand-rolled copy had already
		// drifted from `Apply::Startup::Fn`'s own key names (e.g. `ToolThreshold`
		// vs `tool_threshold_token`) and never honored env var overrides, unlike
		// every other init path in the crate.
		//
		// The globals are a second, separate act: `Apply::Startup::Fn` writes the session's
		// state and nothing else, so the preview cap, the marker grammar and the template
		// pair are published here. They are process-wide, so an init that skipped them
		// left every render point on the previous document's values.
		(!Path.is_empty()).then(|| {
			let Configuration = crate::ConfigurationLoader::Configuration::Struct::Struct::Read(&Path);

			Configuration.Startup(&mut State);
			Configuration.Global();
		});

		Ok(Answer::Text(
			crate::ABI::Handle::Registry::Const::Handles.Allocate(State).to_string(),
		))
	})
}
