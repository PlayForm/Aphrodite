//! `RuntimeHomeOpt`: the runtime home precedence chain.

use std::path::PathBuf;

use crate::Home::{Environment::Path::Fn::Fn as EnvironmentPath, Tilde::Fn::Fn as ExpandTilde};

/// `runtime_home()`, but `None` when no home is resolvable at all (no
/// `$APHRODITE_HOME`, no `$HERMES_HOME`, no `$HOME`, no platform user home).
///
/// Precedence (first non-empty wins), identical to the plugin shim's
/// `_runtime_home()`:
///   1. `$APHRODITE_HOME` - explicit home-level override (expanded); never
///      second-guessed. The shim exports its own decision through this var,
///      so in a plugin process this is the normal path.
///   2. `$HERMES_HOME` + `/aphrodite` - the Hermes home, matching the shim
///      (fixes issue 40: a standalone binary run under a non-default Hermes
///      home lands in the same place the shim would pick).
///   3. `$HOME` + `/.hermes/aphrodite` - the legacy user-data home.
///   4. platform user home + `/.hermes/aphrodite`.
///
/// Getting this order wrong is the issue-40 bug: an `$APHRODITE_HOME` that
/// lost to `$HERMES_HOME`, or a `$HOME` that won over a set `$HERMES_HOME`,
/// silently splits the binary and the plugin across two directories.
pub fn Fn() -> Option<PathBuf> {
	match EnvironmentPath("APHRODITE_HOME") {
		Some(h) => {
			return Some(ExpandTilde(h));
		},
		None => {},
	}

	match EnvironmentPath("HERMES_HOME") {
		Some(h) => {
			return Some(ExpandTilde(h).join("aphrodite"));
		},
		None => {},
	}

	match EnvironmentPath("HOME") {
		Some(h) => {
			return Some(ExpandTilde(h).join(".hermes").join("aphrodite"));
		},
		None => {},
	}

	dirs::home_dir().map(|h| h.join(".hermes").join("aphrodite"))
}
