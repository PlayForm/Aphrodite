//! The two run-under-lock entry points: one keyed by session, one session-less.
//!
//! Neither entry point may block: `with_session` returns `None` on a wedged
//! session so the caller can fail open, and `with_shared` runs the closure
//! against a throwaway template clone instead. A panic inside the closure is
//! NOT caught here - the guard lives one layer up in `Guard::guarded`, which
//! every FFI call site routes through.

use aphrodite::State::Struct::AphroditeState;

use crate::{
	Configuration::Template::Fn::config_template,
	Session::{
		Lock::Fn::lock_bounded,
		Registry::Struct::{DEFAULT_SESSION, session_state},
	},
};

/// Run `f` against the given session's state under a BOUNDED lock.
///
/// Resolves the per-session state for `sid` (gets-or-creates) and locks ONLY
/// that session's mutex - a heavy hook in one session can never block
/// another session's hooks. `None` means the session's lock is wedged by an
/// abandoned worker; the caller MUST degrade fail-open (never block).
pub(crate) fn with_session<T>(sid:&str, f:impl FnOnce(&mut AphroditeState) -> T) -> Option<T> {
	let state = session_state(sid);
	lock_bounded(&state).map(|mut guard| f(&mut guard))
}

/// Run `f` against the session state under a bounded lock.
///
/// Compatibility shim for tools and tests that lack a `session_id`: resolves
/// `crate::Debug::Session::Last::Fn::Fn()` (fallback to the [`DEFAULT_SESSION`] key
/// when empty) and locks ONLY that session with the bounded lock. On `None`
/// (wedged lock) it runs `f` against a THROWAWAY clone of the config
/// template so the call still returns quickly - fail-open, never block; the
/// mutation is discarded (the template itself is never mutated, so future
/// session seeds stay pristine).
///
/// Torn-state contract (report 06 F12): `with_shared` itself does not
/// `catch_unwind` - the panic guard lives one layer up, in `guarded()` at
/// every call site that dispatches into this fn. If `f` panics partway
/// through a multi-step mutation, whatever partial state existed at the
/// panic point is what the next caller observes (the lock itself is fine -
/// poison is recovered via `into_inner` in `lock_bounded`). Keep multi-step
/// mutations (e.g. `retain` then `push_front` in `inline_store_put`) ordered
/// so an interruption degrades to "entry missing" rather than "duplicate
/// entry".
pub(crate) fn with_shared<T>(f:impl FnOnce(&mut AphroditeState) -> T) -> T {
	let sid = crate::Debug::Session::Last::Fn::Fn();
	let sid = match sid.is_empty() {
		true => DEFAULT_SESSION.to_string(),
		false => sid,
	};
	let state = session_state(&sid);

	match lock_bounded(&state) {
		Some(mut guard) => f(&mut guard),
		// Wedged session lock: degrade on a throwaway clone rather than block.
		None => {
			let mut scratch = config_template().clone();
			f(&mut scratch)
		},
	}
}
