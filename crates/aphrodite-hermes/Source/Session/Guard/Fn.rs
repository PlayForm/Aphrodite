//! The test serialiser for the process-global session state.
//!
//! Tests that assert ACROSS multiple state-mutating calls must hold this
//! guard, because the registry is process-global and `on_session_start`
//! resets the default session. It is compiled only under `cfg(test)`; it is
//! not a runtime guard and must never be used outside a test.

use std::sync::{Mutex, OnceLock};

/// Serializes tests that assert across multiple state-mutating calls, since
/// the shared session state is process-global and `on_session_start` resets it.
pub(crate) fn test_guard() -> std::sync::MutexGuard<'static, ()> {
	static G:OnceLock<Mutex<()>> = OnceLock::new();
	G.get_or_init(|| Mutex::new(()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner)
}
