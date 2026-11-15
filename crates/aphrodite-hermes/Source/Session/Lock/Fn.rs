//! The bounded lock: the one acquisition every session-state caller uses.
//!
//! This function is the guard `Maintain/SECURITY.md` names, and the reason the
//! gateway can no longer wedge. It must never be replaced by a plain
//! `Mutex::lock()` and its deadline must never be widened - a caller that
//! blocks here blocks a Hermes hook worker, which is exactly the failure this
//! bound exists to prevent.

use std::sync::{Arc, Mutex};

use aphrodite::State::Struct::AphroditeState;

/// Bounded lock acquisition: try_lock in a loop bounded by a ~100ms
/// wall-clock deadline (the spec's "about 50 x 2ms" budget; a wall-clock
/// deadline keeps the bound precise despite platform sleep-granularity
/// overshoot - macOS sleeps ~6ms for a 2ms request).
///
/// Returns `None` when the mutex is wedged by an abandoned worker thread
/// (the gateway-hang root cause: Hermes' `plugins.hook_callback_timeout`
/// abandoned a worker that still held the old single global lock), so the
/// caller degrades fail-open instead of blocking past the hook timeout.
/// Poison is recovered like the old `.lock()` path: a panic in one call
/// must not wedge every later call.
pub(crate) fn lock_bounded(state:&Arc<Mutex<AphroditeState>>) -> Option<std::sync::MutexGuard<'_, AphroditeState>> {
	let deadline = std::time::Instant::now() + std::time::Duration::from_millis(100);
	loop {
		match state.try_lock() {
			Ok(guard) => return Some(guard),
			// Poisoned: recover the guard (same contract as the pre-fix
			// `.unwrap_or_else(PoisonError::into_inner)` path).
			Err(std::sync::TryLockError::Poisoned(p)) => return Some(p.into_inner()),
			Err(std::sync::TryLockError::WouldBlock) => {
				match std::time::Instant::now() >= deadline {
					true => return None,
					false => std::thread::sleep(std::time::Duration::from_millis(2)),
				}
			},
		}
	}
}
