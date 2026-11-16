//! `AphroditeConfigurationSet` - write one configuration value on the session.

use std::os::raw::c_char;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, State::Fn::Fn as State},
	Configuration::{ApplyUpdate::Fn::Fn as ApplyUpdate, ParseUpdate::Fn::Fn as ParseUpdate},
	Guard::Cstr::{Fn::Fn as Cstr, Optional::Fn::Fn as Optional},
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
/// # Safety
/// `Handle`, `Key` and `Value` must each be null or point to a valid,
/// NUL-terminated C string that stays readable and unmoved for the duration of
/// this call. The returned pointer is owned by the caller and must be released
/// with `AphroditeFreeString`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AphroditeConfigurationSet(
	Handle:*const c_char,
	Key:*const c_char,
	Value:*const c_char,
) -> *mut c_char {
	let Key = unsafe { Cstr(Key) };
	let Value = unsafe { Cstr(Value) };

	// Resolved and parsed before the lock: an absent or oversize argument and an
	// unparseable number are all decided here, so the session lock is held for a
	// field write and nothing else. The refusal is carried to the closure rather
	// than answered here, because the boundary owns the envelope.
	let Parsed = match (Optional(Key), Optional(Value)) {
		(Ok(Key), Ok(Value)) => Ok(ParseUpdate(&Key, &Value)),
		(Err(Error), _) => Err(Error),
		(_, Err(Error)) => Err(Error),
	};

	State(Handle, move |s| {
		let _Outcome = ApplyUpdate(s, Parsed?);

		Ok(Answer::JSON(serde_json::json!({"status":"ok"})))
	})
}
