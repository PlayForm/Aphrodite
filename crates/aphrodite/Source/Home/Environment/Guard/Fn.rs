//! The crate-wide env-var test mutex.

use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

/// Serializes tests that mutate process-global env vars (`HERMES_HOME`,
/// `APHRODITE_HOME`, `HOME`). ONE shared mutex for the whole crate - a test
/// in `prefetch.rs` must never observe a transient `HERMES_HOME` written by
/// a concurrent `home.rs` test (separate per-module guards race: the refusal
/// check compares against `hermes_home()`, so a stray override silently
/// turns a refused path into `Missing`).
///
/// Test-only by construction: it is `#[cfg(test)]`, so a release build has no
/// mutex here at all and the resolvers stay allocation-free.
pub(crate) fn Fn() -> MutexGuard<'static, ()> {
	static G:OnceLock<Mutex<()>> = OnceLock::new();
	G.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(PoisonError::into_inner)
}
