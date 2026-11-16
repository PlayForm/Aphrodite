//! `State`: a handle argument plus one fallible session operation.

use std::os::raw::c_char;

use crate::State::Struct::AphroditeState;

/// Decode the handle and run `Operation` against that session, under
/// `Handle::Lookup::Guarded`.
///
/// This is the shape every mutating export shares: the closure is one fallible
/// operation and the boundary owns the envelope, the panic and the ownership
/// transfer. `Guarded` adds its own `catch_unwind`, so a panic inside
/// `Operation` answers `"internal error: hook panicked"` - the wording these
/// exports have always had.
pub(crate) fn Fn(
	Handle:*const c_char,
	Operation:impl FnOnce(&mut AphroditeState) -> anyhow::Result<crate::ABI::Boundary::Answer::Enum::Enum>,
) -> *mut c_char {
	let Identity = unsafe { crate::ABI::Boundary::Handle::Fn::Fn(Handle) };

	crate::ABI::Boundary::Run::Fn::Fn(move || {
		let Identity = Identity.ok_or_else(|| anyhow::anyhow!("invalid handle"))?;

		crate::ABI::Handle::Lookup::Guarded(Identity, Operation)
	})
}
