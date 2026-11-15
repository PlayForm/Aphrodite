//! What the registration command did.

/// The outcome of `hermes plugins enable aphrodite`, as a value.
///
/// Registration used to return `anyhow::Result<()>`: a spawn failure became an
/// error and a non-zero exit was warned away inside the same function, so the only
/// thing a caller could learn was "no error" - which is true both for a command that
/// exited zero and for one that was never run because the plugin directory was
/// report-only.
///
/// The three variants are the three things this step can actually observe. The
/// fourth question - does Hermes then *load* the plugin - is deliberately not a
/// variant: setup never reads the plugin list back, so a variant claiming it would
/// be a claim the code cannot support. `Execute::Registration` says so in the
/// output instead of asserting it.
///
/// Nothing here is named `Verified` or `Installed`, because none of these outcomes
/// means either.
#[derive(Clone)]
pub(crate) enum Enum {
	/// The `hermes` CLI could not be started at all: nothing was registered.
	NotStarted {
		/// Why the process could not be started.
		Cause:String,
	},
	/// The CLI started and exited non-zero. It has already told the user why on
	/// stderr; the plugin may still be discovered on Hermes' next scan.
	ExitedNonZero {
		/// The exit code, or `None` when the process was killed by a signal.
		Code:Option<i32>,
		/// The command's own stderr, verbatim.
		Stderr:String,
	},
	/// The CLI started and exited zero. The command succeeded; whether the plugin
	/// loads is not known here.
	ExitedZero,
}
