//! `AphroditeDirective` - inspect and mutate the session's active directives.

use std::os::raw::c_char;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, State::Bare::Fn::Fn as Bare},
	Guard::Cstr::Fn::Fn as Cstr,
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
	let Action = unsafe { Cstr(Action) }.unwrap_or_default();
	let name = unsafe { Cstr(name) }.unwrap_or_default();

	Bare(Handle, move |s| {
		Ok(Answer::Envelope(crate::Directive::Action::Fn(s, &Action, &name)))
	})
}
