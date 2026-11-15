//! Handle lookup and the stateful-call wrapper.
//!
//! Two layers, deliberately separate: `GetHandle` resolves an ID to a session
//! and drops the map lock before returning, `WithState` takes that session's
//! own lock and runs a closure under the panic boundary. Every export that
//! mutates session state goes through both.

use std::sync::{Arc, Mutex};

use super::Table::Handles;
use crate::State::Struct::AphroditeState;

/// Look up a session's handle and clone its `Arc` out from under the map
/// lock, which is dropped before returning - callers lock the per-session
/// mutex separately so the map lock is never held across state access.
pub(crate) fn GetHandle(Hid:usize) -> Option<Arc<Mutex<AphroditeState>>> {
	Handles().as_ref().and_then(|m| m.get(&Hid)).cloned()
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
pub(crate) fn WithState<T>(Hid:usize, f:impl FnOnce(&mut AphroditeState) -> T) -> Result<T, String> {
	let Session = match GetHandle(Hid) {
		Some(s) => s,
		None => return Err(format!("invalid handle: {}", Hid)),
	};
	let mut State = Session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
	std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&mut State)))
		.map_err(|_| "internal error: hook panicked".to_string())
}
