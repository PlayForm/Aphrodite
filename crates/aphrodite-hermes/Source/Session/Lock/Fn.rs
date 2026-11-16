//! The bounded lock: the one acquisition every session-state caller uses.
//!
//! This function is the guard `Maintain/SECURITY.md` names, and the reason the
//! gateway can no longer wedge. It must never be replaced by a plain
//! `Mutex::lock()` and its deadline must never be widened - a caller that
//! blocks here blocks a Hermes hook worker, which is exactly the failure this
//! bound exists to prevent.
//!
//! It answers with [`Outcome::Enum::Enum`] rather than `Option`, so "the lock is held
//! by an abandoned worker" is a named state a caller can count, report or
//! degrade from - never a bare `None` that means whatever the caller's own
//! `None` already meant.

use std::sync::{Arc, Mutex};

use aphrodite::State::Struct::AphroditeState;

use crate::Session::Lock::Outcome;

/// Bounded lock acquisition: try_lock in a loop bounded by a ~100ms
/// wall-clock deadline (the spec's "about 50 x 2ms" budget; a wall-clock
/// deadline keeps the bound precise despite platform sleep-granularity
/// overshoot - macOS sleeps ~6ms for a 2ms request).
///
/// Returns `Outcome::Enum::Enum::Wedged` when the mutex is still held at the deadline
/// (the gateway-hang root cause: Hermes' `plugins.hook_callback_timeout`
/// abandoned a worker that still held the old single global lock), so the
/// caller degrades fail-open instead of blocking past the hook timeout.
///
/// Poison is recovered into a guard, not reported: a panic in one call must not
/// wedge every later call, and a session whose state survived the panic is more
/// useful readable than declared dead.
pub(crate) fn lock_bounded<'a>(state:&'a Arc<Mutex<AphroditeState>>) -> Outcome::Enum::Enum<'a> {
	let deadline = std::time::Instant::now() + std::time::Duration::from_millis(100);
	loop {
		match state.try_lock() {
			Ok(guard) => return Outcome::Enum::Enum::Held(guard),
			// Poisoned: recover the guard (same contract as the pre-fix
			// `.unwrap_or_else(PoisonError::into_inner)` path).
			Err(std::sync::TryLockError::Poisoned(p)) => return Outcome::Enum::Enum::Held(p.into_inner()),
			Err(std::sync::TryLockError::WouldBlock) => {
				match std::time::Instant::now() >= deadline {
					true => return Outcome::Enum::Enum::Wedged,
					false => std::thread::sleep(std::time::Duration::from_millis(2)),
				}
			},
		}
	}
}
