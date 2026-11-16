//! Handle lookup and the stateful-call wrapper.
//!
//! Two entry points, deliberately separate: `Locked` resolves an ID to a
//! session, takes that session's own lock and runs a fallible closure under it;
//! `WithState` is `Locked` plus the panic boundary. Every export that touches
//! session state goes through one of them, and neither ever holds the table
//! lock.

use crate::State::Struct::AphroditeState;

/// Look up a session's handle and clone its `Arc` out from under the table
/// lock, which is dropped before returning - callers lock the per-session
/// mutex separately so the table lock is never held across state access.
pub(crate) fn GetHandle(Hid:usize) -> Option<crate::ABI::Handle::Registry::Struct::Session> {
	crate::ABI::Handle::Registry::Const::Handles.Resolve(Hid)
}

/// Resolve `Hid` and run `f` under its session lock, without a panic guard.
///
/// An unknown handle is an error, not a default: silently inventing a session
/// would answer with plausible JSON for a session that never existed.
///
/// This is the half `WithState` is built from, and it exists as its own entry
/// point because the read-shaped exports answer a different panic string - see
/// `ABI::Boundary::State::Bare`.
pub(crate) fn Locked<T>(Hid:usize, f:impl FnOnce(&mut AphroditeState) -> anyhow::Result<T>) -> anyhow::Result<T> {
	let Session = GetHandle(Hid).ok_or_else(|| anyhow::anyhow!("invalid handle: {Hid}"))?;
	let mut State = Session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);

	f(&mut State)
}

/// Torn-state contract (report 06 F12): `f` runs under `catch_unwind`, so a
/// panicking hook returns an error instead of aborting the process, but any
/// mutation `f` already made to `State` before panicking is NOT rolled back.
/// A panic between two steps of a multi-step mutation (e.g. `retain` then
/// `push_front` in `InlineStorePut`) leaves whatever partial state existed
/// at the panic point for the next caller to observe. This crate's mutation
/// sequences are ordered so an interruption degrades to "entry missing"
/// rather than "duplicate entry" - keep that ordering if you touch them.
///
/// An unknown handle is an error, not a default: silently inventing a session
/// would answer with plausible JSON for a session that never existed.
///
/// The catch encloses the resolution and the lock as well as `f`. Nothing in
/// those two steps can panic - the poison case is recovered, not unwrapped - so
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
pub(crate) fn WithState<T>(Hid:usize, f:impl FnOnce(&mut AphroditeState) -> anyhow::Result<T>) -> anyhow::Result<T> {
	std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Locked(Hid, f)))
		.map_err(|Payload| crate::ABI::Boundary::Panic::Fn::Fn(Payload, "internal error: hook panicked"))
		.and_then(|Inner| Inner)
}
