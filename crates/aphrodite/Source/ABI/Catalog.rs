//! `AphroditeCatalog` - list the session's stored entries as a catalogue.

use std::os::raw::c_char;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, State::Bare::Fn::Fn as Bare},
	Guard::Cstr::Fn::Fn as Cstr,
};

/// Answer with the session's catalogue in the requested `Mode`.
///
/// The catalogue is what a caller reads when it remembers the content but not
/// the hash; `Mode` selects the verbosity (`full` when the caller passes
/// something unrecognised). It is a read: nothing is marked retrieved and no
/// teaching signal is recorded, which is exactly why a caller that then expands
/// an entry still has to retrieve it.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeCatalog(Handle:*const c_char, Mode:*const c_char) -> *mut c_char {
	let Mode = unsafe { Cstr(Mode) }.unwrap_or_default();

	Bare(Handle, move |s| Ok(Answer::Envelope(crate::Catalog::Build::Fn::Fn(s, &Mode))))
}
