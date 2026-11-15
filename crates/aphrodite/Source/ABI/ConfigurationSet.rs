//! `AphroditeConfigurationSet` - write one configuration value on the session.

use std::os::raw::c_char;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, State::Fn::Fn as State},
	Configuration::{ApplyUpdate::Fn::Fn as ApplyUpdate, ParseUpdate::Fn::Fn as ParseUpdate},
	Guard::Cstr::Fn::Fn as Cstr,
};

/// Set one configuration value and answer with `{"status":"ok"}`.
///
/// Silently ignores an unknown key and an unparseable number: this is a runtime
/// knob, not a validated form, and a host that mistypes a key must not have its
/// session fail. The cost of that leniency is that a caller cannot tell a
/// rejected write from an applied one by the response alone - it has to read the
/// value back with `AphroditeConfigurationGet`.
///
/// "Silently" is about the response, not about the code: `ParseUpdate` answers a
/// typed `Outcome` naming which of the two happened, so a reader of this file
/// learns the acceptance rules from the type instead of from a `_ => {}` arm.
/// The envelope stays `ok` in all three cases because it always has.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeConfigurationSet(
	Handle:*const c_char,
	Key:*const c_char,
	Value:*const c_char,
) -> *mut c_char {
	let Key = unsafe { Cstr(Key) }.unwrap_or_default();
	let Value = unsafe { Cstr(Value) }.unwrap_or_default();

	// Parsed before the lock: an unparseable number and an unknown key are
	// decided here, so the session lock is held for a field write and nothing
	// else.
	let Parsed = ParseUpdate(&Key, &Value);

	State(Handle, move |s| {
		let _Outcome = ApplyUpdate(s, Parsed);

		Ok(Answer::JSON(serde_json::json!({"status":"ok"})))
	})
}
