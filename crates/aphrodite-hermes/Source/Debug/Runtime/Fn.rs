//! The runtime home every debug artifact resolves against.
//!
//! One decision, shared by the flag files, the persisted session id and the
//! core logger's scratch home. Getting it wrong is silent: a flag written into
//! a shadow home is simply never read, and the toggle appears to do nothing.

use std::path::PathBuf;

/// Resolve the runtime home for debug artifacts.
///
/// `$APHRODITE_DEBUG_HOME` (dev-only override, set by tests so the flag reader
/// agrees with the core logger's scratch home) wins; otherwise the shared
/// runtime-home decision (issue 40): `$APHRODITE_HOME` -> `$HERMES_HOME` ->
/// `$HOME` -> platform default. Under a non-default Hermes home (Docker,
/// profiles) the debug flags land in that home, not in a shadow
/// `$HOME/.hermes/aphrodite`.
///
/// Test builds never touch the real home: they resolve to a fixed scratch
/// directory so the live session's toggle and `session.current` can neither
/// leak into nor be polluted by the test process.
///
/// It does not create the home - callers that write are responsible for the
/// directory existing, and a failed write degrades to "no persistence".
pub(crate) fn Fn() -> PathBuf {
	match std::env::var("APHRODITE_DEBUG_HOME") {
		Ok(home) if !home.is_empty() => return PathBuf::from(home),
		_ => {},
	}

	#[cfg(test)]
	{
		// Hermetic tests: route debug flags / session persistence away from
		// the real runtime home so the live session's toggle and
		// `session.current` can never leak into (or be polluted by) the
		// test process.
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
