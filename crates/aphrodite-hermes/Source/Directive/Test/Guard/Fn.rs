//! Serialization guard for tests that mutate process-global environment
//! variables.
//!
//! `APHRODITE_HOME` / `APHRODITE_DIRECTIVES_DIR` / `HERMES_HOME` / `HOME` are
//! process-wide, and the harness runs tests in parallel threads, so two tests
//! that set them concurrently observe each other's values. Holding this guard
//! for the whole test is the only thing that makes the precedence assertions
//! mean anything.

/// Serializes tests that mutate process-global env vars.
///
/// It is not reentrant and not poisoned-on-panic: the mutex is recovered with
/// `into_inner`, so one failing test cannot wedge every later one.
pub(crate) fn Fn() -> std::sync::MutexGuard<'static, ()> {
	static G:std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
	G.get_or_init(|| std::sync::Mutex::new(()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner)
}
