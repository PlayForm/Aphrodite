//! `DebugRuntimeHome`: the directory the log is written into.

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
pub(crate) fn Fn() -> PathBuf {
	if let Ok(Home) = std::env::var("APHRODITE_DEBUG_HOME")
		&& !Home.is_empty()
	{
		return PathBuf::from(Home);
	}
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
}
