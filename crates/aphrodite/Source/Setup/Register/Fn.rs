//! Running the plugin-registration command.

use std::process::Command;

use super::Outcome;

/// Run `hermes plugins enable aphrodite` and report what happened.
///
/// The status is matched on its **exit code**, not on `success()`: `Some(0)` is the
/// only success, a signal death has no code, and any other code is the command
/// refusing. That is one match over the value the process actually produced, where
/// `success()` would collapse the three cases into a boolean and force a second
/// branch to recover them.
///
/// It is not verification and it does not present anything: the caller decides
/// whether `NotStarted` fails the run and how to render the other two.
pub(crate) fn Fn() -> Outcome::Enum::Enum {
	match Command::new("hermes").args(["plugins", "enable", "aphrodite"]).output() {
		Ok(Out) => {
			match Out.status.code() {
				Some(0) => Outcome::Enum::Enum::Zero,
				Code => Outcome::Enum::Enum::NonZero { Code, Stderr:String::from_utf8_lossy(&Out.stderr).into_owned() },
			}
		},
		Err(e) => Outcome::Enum::Enum::NotStarted { Cause:e.to_string() },
	}
}
