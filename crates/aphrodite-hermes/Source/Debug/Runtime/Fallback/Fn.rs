//! The runtime home when `$APHRODITE_DEBUG_HOME` is unset or empty.

use std::path::PathBuf;

/// The shared runtime-home decision (issue 40): `$APHRODITE_HOME` ->
/// `$HERMES_HOME` -> `$HOME` -> platform default. Under a non-default Hermes
/// home (Docker, profiles) the debug flags land in that home, not in a shadow
/// `$HOME/.hermes/aphrodite`.
///
/// Test builds never touch the real home: they resolve to a fixed scratch
/// directory so the live session's toggle and `session.current` can neither
/// leak into nor be polluted by the test process.
///
/// It does not create the home - callers that write are responsible for the
/// directory existing, and a failed write degrades to "no persistence".
pub(crate) fn Fn() -> PathBuf {
	#[cfg(test)]
	{
		// Hermetic tests: route debug flags / session persistence away from the
		// real runtime home so the live session's toggle and `session.current`
		// can never leak into (or be polluted by) the test process.
		let scratch = std::env::temp_dir().join("aphrodite-hermes-test-home");
		let _ = std::fs::create_dir_all(&scratch);
		scratch
	}
	#[cfg(not(test))]
	{
		aphrodite::Home::Runtime::Opt::Fn::Fn()
			.unwrap_or_else(|| std::env::temp_dir().join(".hermes").join("aphrodite"))
	}
}
