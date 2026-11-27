//! Bounded mutex acquisition, shared by the core engine and the Hermes bridge.
//!
//! The gateway hosts ONE plugin instance for every session (parent, subagents,
//! delegated tasks). Hermes abandons a hook worker that outlives
//! `plugins.hook_callback_timeout`, and an abandoned worker keeps whatever
//! mutex it held - forever. A plain `Mutex::lock()` on that mutex therefore
//! parks every later hook dispatch until the gateway is restarted ("3
//! abandoned workers - plugin is hung").
//!
//! [`bounded`] is the treatment: `try_lock` in a loop behind a wall-clock
//! deadline (the spec's "about 50 x 2ms" budget, kept as a deadline because a
//! 2ms sleep overshoots to ~6ms on macOS). Callers MUST treat `None` as
//! "degrade fail-open", never as "retry": a wedged debug/telemetry lock may
//! skip its update, a wedged state lock may run on a throwaway clone, but a
//! hook must always return.

use std::{
	sync::{Mutex, MutexGuard, TryLockError},
	time::{Duration, Instant},
};

/// Wall-clock budget for one bounded acquisition: long enough to absorb
/// ordinary contention (a hook finishing its own state mutation), short
/// enough that a wedged lock costs a hook nothing close to its timeout.
const DEADLINE:Duration = Duration::from_millis(100);

/// Sleep between `try_lock` attempts.
const RETRY:Duration = Duration::from_millis(2);

/// Acquire `mutex` within [`DEADLINE`], or return `None` when it is wedged.
///
/// Poison is recovered (`PoisonError::into_inner`), matching the plain
/// `.lock().unwrap_or_else(PoisonError::into_inner)` sites this replaces: a
/// panic in one call must not wedge every later call.
pub fn bounded<T>(mutex:&Mutex<T>) -> Option<MutexGuard<'_, T>> {
	let deadline = Instant::now() + DEADLINE;
	loop {
		match mutex.try_lock() {
			Ok(guard) => return Some(guard),
			Err(TryLockError::Poisoned(poisoned)) => return Some(poisoned.into_inner()),
			Err(TryLockError::WouldBlock) => {
				if Instant::now() >= deadline {
					return None;
				}
				std::thread::sleep(RETRY);
			},
		}
	}
}
