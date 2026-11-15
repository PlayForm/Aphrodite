//! `Bare`: a handle argument plus one fallible session operation, without
//! `WithState`'s inner panic guard.

use std::os::raw::c_char;

use crate::State::Struct::AphroditeState;

/// Decode the handle and run `Operation` against that session under the
/// session lock alone, leaving the panic to `Boundary::Run`.
///
/// The difference from `State::Fn` is one observable string, and it is the
/// reason this leaf exists: the read-shaped exports
/// (`AphroditeRetrieve`, `AphroditeCatalog`, `AphroditeStats`,
/// `AphroditeSearch`, `AphroditeDirective`, `AphroditeConfigurationGet`)
/// resolved the handle and took the session lock themselves and were then
/// wrapped in the outer panic boundary, so a panic in one of them answers
/// `"internal error: panicked"`, not `"internal error: hook panicked"`. Both
/// messages are contract strings a host may match on, so the pass that gave
/// them one boundary must not also give them one wording.
///
/// `Handle::Lookup::Locked` is the same resolution and locking as `WithState`
/// with the `catch_unwind` removed, so the two differ in nothing else.
pub(crate) fn Fn(
	Handle:*const c_char,
	Operation:impl FnOnce(&mut AphroditeState) -> anyhow::Result<crate::ABI::Boundary::Answer::Enum::Enum>,
) -> *mut c_char {
	let Hid = unsafe { crate::ABI::Boundary::Handle::Fn::Fn(Handle) };

	crate::ABI::Boundary::Run::Fn::Fn(move || {
		let Hid = Hid.ok_or_else(|| anyhow::anyhow!("invalid handle"))?;

		crate::ABI::Handle::Lookup::Locked(Hid, Operation)
	})
}
