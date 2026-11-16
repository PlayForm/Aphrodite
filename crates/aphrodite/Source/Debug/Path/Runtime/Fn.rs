//! `Runtime`: the directory the log is written into.

use std::path::PathBuf;

/// The debug runtime home: $APHRODITE_DEBUG_HOME (dev-only override, used
/// by tests to keep probe files out of the real runtime home) -> the real
/// runtime home in production -> a scratch dir under the OS temp in tests.
/// The scratch fallback matters only when this crate is the test crate
/// itself; as a dependency it compiles WITHOUT cfg(test), so test isolation
/// relies on the env override (set by the hermes test suite).
///
/// The override is checked BEFORE the cfg(test) branch on purpose: a test that
/// sets it wants its own directory even in a test build, and the scratch dir is
/// only the fallback for a test that forgot to.
///
/// `filter` rather than a nested emptiness test: a set-but-empty variable is the
/// same "no override" answer as an unset one, so it produces no `Some` at all.
pub(crate) fn Fn() -> PathBuf {
	let Override = std::env::var("APHRODITE_DEBUG_HOME")
		.ok()
		.filter(|Home| !Home.is_empty())
		.map(PathBuf::from);

	Override.unwrap_or_else(|| {
		#[cfg(test)]
		{
			let Scratch = std::env::temp_dir().join("aphrodite-test-debug-home");
			let _ = std::fs::create_dir_all(&Scratch);

			Scratch
		}
		#[cfg(not(test))]
		{
			crate::Home::Runtime::Fn::Fn()
		}
	})
}
