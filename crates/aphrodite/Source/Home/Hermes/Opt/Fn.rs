//! `HermesHomeOpt`: the Hermes home, or `None` when nothing resolves.

use std::path::PathBuf;

use crate::Home::{Environment::Path::Fn::Fn as EnvironmentPath, Tilde::Fn::Fn as ExpandTilde};

/// `hermes_home()`, but `None` when nothing at all resolves (no env var and
/// no platform user home) so callers that must fail loudly (e.g. `aphrodite
/// setup`) can distinguish that from the degraded `.` fallback.
///
/// `$HERMES_HOME` wins outright and is tilde-expanded; there is no fallback
/// to `$HOME/.hermes` while it is set, because a Hermes home that disagrees
/// with the shim's is exactly the issue-40 failure this module exists to
/// prevent.
pub fn Fn() -> Option<PathBuf> {
	if let Some(h) = EnvironmentPath("HERMES_HOME") {
		return Some(ExpandTilde(h));
	}
	dirs::home_dir().map(|h| h.join(".hermes"))
}
