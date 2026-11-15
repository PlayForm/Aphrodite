//! `AphroditeDirective` - inspect and mutate the session's active directives.

use std::os::raw::c_char;

use super::{
	Guard::{Cstr, Guarded, ToJSONError, ToJSONOk},
	Handle::GetHandle,
};

/// Apply a directive `Action` named by `name` and answer with the action's JSON
/// result.
///
/// The six actions are `list`, `swap`, `remove`, `Load`, `add` and `reset`, and
/// the vocabulary lives in `Directive::Action::Fn` rather than here: this
/// export is only the transport. An unknown action is answered there with an
/// error instead of being silently ignored, and `Load` on a name that does not
/// exist errors where `add` stays silent - the difference matters to a caller
/// treating a directive name as validated.
///
/// Directives are session state, so the effect is visible to every later hook
/// call on this handle and to no other.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeDirective(Handle:*const c_char, Action:*const c_char, name:*const c_char) -> *mut c_char {
	let Hid = match unsafe { Cstr(Handle) }.unwrap_or_default().parse::<usize>() {
		Ok(ID) => ID,
		Err(_) => return ToJSONError("invalid handle"),
	};
	let Act = unsafe { Cstr(Action) }.unwrap_or_default();
	let Nm = unsafe { Cstr(name) }.unwrap_or_default();
	Guarded(std::panic::AssertUnwindSafe(move || {
		match GetHandle(Hid) {
			Some(Session) => {
				let mut s = Session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
				ToJSONOk(&crate::Directive::Action::Fn(&mut s, &Act, &Nm))
			},
			None => ToJSONError(&format!("invalid handle: {}", Hid)),
		}
	}))
}
