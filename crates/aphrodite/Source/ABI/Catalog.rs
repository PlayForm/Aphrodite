//! `AphroditeCatalog` - list the session's stored entries as a catalogue.

use std::os::raw::c_char;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, State::Bare::Fn::Fn as Bare},
	Guard::Cstr::{Fn::Fn as Cstr, Optional::Fn::Fn as Optional},
};

/// Answer with the session's catalogue in the requested `Mode`.
///
/// The catalogue is what a caller reads when it remembers the content but not
/// the hash; `Mode` selects the verbosity (`full` when the caller passes
/// something unrecognised). It is a read: nothing is marked retrieved and no
/// teaching signal is recorded, which is exactly why a caller that then expands
/// an entry still has to retrieve it.
/// # Safety
/// `Handle` and `Mode` must each be null or point to a valid, NUL-terminated C
/// string that stays readable and unmoved for the duration of this call. The
/// returned pointer is owned by the caller and must be released with
/// `AphroditeFreeString`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AphroditeCatalog(Handle:*const c_char, Mode:*const c_char) -> *mut c_char {
	let Mode = unsafe { Cstr(Mode) };

	Bare(Handle, move |s| {
		let Mode = Optional(Mode)?;

		Ok(Answer::JSON(crate::Catalog::Build::Fn::Fn(s, &Mode)))
	})
}
