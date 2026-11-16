//! Handle lookup and the stateful-call wrappers.
//!
//! Three entry points, deliberately separate: `Get` resolves an identity to a
//! session and nothing else; `Locked` is `Get` plus that session's own lock and a
//! fallible closure under it; `Guarded` is `Locked` plus the panic boundary.
//! Every export that touches session state goes through one of them, and none
//! ever holds the table lock.

use crate::State::Struct::AphroditeState;

/// Look up a session's handle and clone its `Arc` out from under the table
/// lock, which is dropped before returning - callers lock the per-session
/// mutex separately so the table lock is never held across state access.
///
/// `Identity` is the decimal value the caller received from `AphroditeInit`;
/// `None` means no live session has that identity.
pub(crate) fn Get(Identity:usize) -> Option<crate::ABI::Handle::Registry::Struct::Session> {
	crate::ABI::Handle::Registry::Const::Handles.Resolve(Identity)
}

/// Resolve `Identity` and run `Operation` under its session lock, without a
/// panic guard.
///
/// An unknown handle is an error, not a default: silently inventing a session
/// would answer with plausible JSON for a session that never existed.
///
/// This is the half `Guarded` is built from, and it exists as its own entry
/// point because the read-shaped exports answer a different panic string - see
/// `ABI::Boundary::State::Bare`.
pub(crate) fn Locked<T>(
	Identity:usize,
	Operation:impl FnOnce(&mut AphroditeState) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
	let Session = Get(Identity).ok_or_else(|| anyhow::anyhow!("invalid handle: {Identity}"))?;
	let mut State = Session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);

	Operation(&mut State)
}

/// Torn-state contract (report 06 F12): `Operation` runs under `catch_unwind`, so
/// a panicking hook returns an error instead of aborting the process, but any
/// mutation `Operation` already made to `State` before panicking is NOT rolled
/// back. A panic between two steps of a multi-step mutation (e.g. `retain` then
/// `push_front` in `State::Store::Put::Put`) leaves whatever partial state existed
/// at the panic point for the next caller to observe. This crate's mutation
/// sequences are ordered so an interruption degrades to "entry missing"
/// rather than "duplicate entry" - keep that ordering if you touch them.
///
/// An unknown handle is an error, not a default: silently inventing a session
/// would answer with plausible JSON for a session that never existed.
///
/// External contract: a panic inside `Operation` answers the C string
/// `"internal error: hook panicked"`, which a host may match on; the read-shaped
/// exports that use `Locked` instead answer `"internal error: panicked"`.
///
/// The catch encloses the resolution and the lock as well as `Operation`. Nothing
/// in those two steps can panic - the poison case is recovered, not unwrapped - so
/// the widened scope is not observable, and it is what lets `Locked` be the one
/// resolution path instead of a second copy of it.
///
/// A panic here also POISONS the session mutex: `Locked` takes the guard inside
/// this catch, so the unwind drops a held guard before reaching it. The next
/// caller recovers the poison rather than failing, which is why a panicking hook
/// degrades one session instead of wedging it - and why the torn-state contract
/// above is the real statement of what a caller observes, not the poison flag.
///
/// The payload is handed to `Boundary::Panic` rather than dropped here: see that
/// leaf for why a caught payload's destructor must not run outside its catch.
pub(crate) fn Guarded<T>(
	Identity:usize,
	Operation:impl FnOnce(&mut AphroditeState) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
	std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Locked(Identity, Operation)))
		.map_err(|Payload| crate::ABI::Boundary::Panic::Fn::Fn(Payload, "internal error: hook panicked"))
		.and_then(|Inner| Inner)
}
