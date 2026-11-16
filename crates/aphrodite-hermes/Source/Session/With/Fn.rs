//! The two run-under-lock entry points: one keyed by session, one session-less.
//!
//! Neither entry point may block: `with_session` returns `None` on a wedged
//! session so the caller can fail open, and `with_shared` runs the closure
//! against a throwaway template clone instead. A panic inside the closure is
//! NOT caught here - the guard lives one layer up in `Guard::guarded`, which
//! every FFI call site routes through.
//!
//! Both are deliberately shaped for the caller that has a VALUE to return and
//! nothing to report: a tool call answers with its result, and a wedged lock is
//! a fail-open degradation it cannot act on. A caller that must know what
//! happened - the config sweep, which has to tell "every session is wedged"
//! from "every session said stop" - uses `Session::Broadcast`, which reports
//! `Applied` and `Skipped` counts instead of collapsing both into one value.

use aphrodite::State::Struct::AphroditeState;

use crate::{
	Configuration::Template,
	Session::{
		Lock::{self, Outcome},
		Registry::{Const::Fallback, Get},
	},
};

/// Run `f` against the given session's state under a BOUNDED lock.
///
/// Resolves the per-session state for `sid` (gets-or-creates) and locks ONLY
/// that session's mutex - a heavy hook in one session can never block
/// another session's hooks. `None` means the session's lock is wedged by an
/// abandoned worker; the caller MUST degrade fail-open (never block).
///
/// `with_session` keeps its qualifier until the callers outside this partition
/// (`Hook::Call::Dispatch`, which names this leaf) repoint at
/// `Session::With::Session::Fn`.
pub(crate) fn with_session<T>(sid:&str, f:impl FnOnce(&mut AphroditeState) -> T) -> Option<T> {
	let State = Get::Fn::Fn(sid);

	match Lock::Fn::Fn(&State) {
		Outcome::Enum::Enum::Held(mut Guard) => Some(f(&mut Guard)),
		Outcome::Enum::Enum::Wedged => None,
	}
}

/// Run `f` against the session state under a bounded lock.
///
/// Compatibility shim for tools and tests that lack a `session_id`: resolves
/// `crate::Debug::Session::Last::Fn::Fn()` (fallback to the
/// [`Fallback`](crate::Session::Registry::Const::Fallback) key when empty) and
/// locks ONLY that session with the bounded lock. On a wedged lock it runs `f`
/// against a THROWAWAY clone of the config template so the call still returns
/// quickly - fail-open, never block; the mutation is discarded (the template
/// itself is never mutated, so future session seeds stay pristine). The discard
/// is the price of a tool call that has nothing better to do than answer; a
/// caller that needs to know whether its write landed uses `Session::Broadcast`
/// (which counts) or `with_session` (which reports the wedged lock as `None`)
/// instead.
///
/// Torn-state contract (report 06 F12): `with_shared` itself does not
/// `catch_unwind` - the panic guard lives one layer up, in
/// `Guard::Fn::Fn(guarded)` at every call site that dispatches into this fn. If
/// `f` panics partway through a multi-step mutation, whatever partial state
/// existed at the panic point is what the next caller observes (the lock itself
/// is fine - poison is recovered via `into_inner` in `Lock::Fn::Fn`). Keep
/// multi-step mutations (e.g. `retain` then `push_front` in
/// `inline_store_put`) ordered so an interruption degrades to "entry missing"
/// rather than "duplicate entry".
pub(crate) fn with_shared<T>(f:impl FnOnce(&mut AphroditeState) -> T) -> T {
	let Sid = crate::Debug::Session::Last::Fn::Fn();
	let Sid = match Sid.is_empty() {
		true => Fallback.to_string(),
		false => Sid,
	};
	let State = Get::Fn::Fn(&Sid);

	match Lock::Fn::Fn(&State) {
		Outcome::Enum::Enum::Held(mut Guard) => f(&mut Guard),
		// Wedged session lock: degrade on a throwaway clone rather than block.
		Outcome::Enum::Enum::Wedged => {
			let mut Scratch = Template::Fn::Fn().clone();
			f(&mut Scratch)
		},
	}
}
