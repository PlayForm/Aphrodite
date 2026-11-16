//! `Opt`: the Hermes home, or `None` when nothing resolves.

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
///
/// Two alternatives, chained with `or_else`: the platform home is asked only when the
/// environment named nothing, which is the whole content of "wins outright".
pub fn Fn() -> Option<PathBuf> {
	EnvironmentPath("HERMES_HOME")
		.map(ExpandTilde)
		.or_else(|| dirs::home_dir().map(|Home| Home.join(".hermes")))
}
