//! `AphroditeDirective` - inspect and mutate the session's active directives.

use std::os::raw::c_char;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, State::Bare::Fn::Fn as Bare},
	Guard::Cstr::{Fn::Fn as Cstr, Optional::Fn::Fn as Optional},
};

/// Apply a directive `Action` named by `Name` and answer with the action's JSON
/// result.
///
/// The six actions are `list`, `swap`, `remove`, `load`, `add` and `reset`, and
/// the vocabulary lives in `Directive::Action::Fn` rather than here: this
/// export is only the transport. An unknown action is answered there with an
/// error instead of being silently ignored, and `load` on a name that does not
/// exist errors where `add` stays silent - the difference matters to a caller
/// treating a directive name as validated.
///
/// Directives are session state, so the effect is visible to every later hook
/// call on this handle and to no other.
/// # Safety
/// `Handle`, `Action` and `Name` must each be null or point to a valid,
/// NUL-terminated C string that stays readable and unmoved for the duration of
/// this call. The returned pointer is owned by the caller and must be released
/// with `AphroditeFreeString`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AphroditeDirective(
	Handle:*const c_char,
	Action:*const c_char,
	Name:*const c_char,
) -> *mut c_char {
	let Action = unsafe { Cstr(Action) };
	let Name = unsafe { Cstr(Name) };

	Bare(Handle, move |s| {
		let Action = Optional(Action)?;
		let Name = Optional(Name)?;

		Ok(Answer::JSON(crate::Directive::Action::Fn(s, &Action, &Name)))
	})
}
