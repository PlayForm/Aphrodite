//! `AphroditeRetrieve` - hand back the stored payload behind a hash.

use std::os::raw::c_char;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, State::Bare::Fn::Fn as Bare},
	Guard::Cstr::{Fn::Fn as Cstr, Optional::Fn::Fn as Optional},
};

/// Return the raw content stored under `Hash` - not a JSON envelope.
///
/// This is the one stateful export whose success payload is the content itself,
/// because the caller's next move is to put it back into context verbatim; a
/// JSON wrapper would have to be unescaped by every host. Failures are still
/// `{"error": ...}` JSON, so a caller must distinguish "content that happens to
/// start with a brace" from an error by checking for the `error` key, not by
/// shape alone.
///
/// Retrieval is also the teaching loop: the FFI path bypasses `Resolve::One::Fn`,
/// so the consequence is attributed here instead, which is what lets the engine
/// learn which markers are actually worth keeping.
/// # Safety
/// `Handle` and `Hash` must each be null or point to a valid, NUL-terminated C
/// string that stays readable and unmoved for the duration of this call. The
/// returned pointer is owned by the caller and must be released with
/// `AphroditeFreeString`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AphroditeRetrieve(Handle:*const c_char, Hash:*const c_char) -> *mut c_char {
	let Hash = unsafe { Cstr(Hash) };

	Bare(Handle, move |s| {
		let Hash = Optional(Hash)?;

		match s.Store.Get(&Hash) {
			Some(Content) => {
				// Tier 1 teaching loop: the FFI retrieve path bypasses
				// `Resolve::One::Fn`, so attribute the consequence here too.
				s.ChainSplit.Retrieve(&Hash);

				Ok(Answer::Text(Content))
			},
			None => anyhow::bail!("hash not found: {}", Hash),
		}
	})
}
