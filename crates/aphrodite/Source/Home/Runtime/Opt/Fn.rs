//! `Opt`: the runtime home precedence chain.

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
///
/// The chain is `or_else`, so each environment read happens only when every
/// higher-priority one was absent - which is what a precedence chain means. It used
/// to be four `match` statements with an early `return` in the `Some` arm and an
/// empty `None` arm, which read as four independent decisions and made the order a
/// property of the statement sequence rather than of the value.
pub fn Fn() -> Option<PathBuf> {
	EnvironmentPath("APHRODITE_HOME")
		.map(ExpandTilde)
		.or_else(|| EnvironmentPath("HERMES_HOME").map(|Home| ExpandTilde(Home).join("aphrodite")))
		.or_else(|| EnvironmentPath("HOME").map(|Home| ExpandTilde(Home).join(".hermes").join("aphrodite")))
		.or_else(|| dirs::home_dir().map(|Home| Home.join(".hermes").join("aphrodite")))
}
