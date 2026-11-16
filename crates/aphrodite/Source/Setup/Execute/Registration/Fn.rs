//! Presenting the registration outcome and deciding whether it fails the run.

use crate::Setup::Register;

/// Register the plugin, then render what happened.
///
/// The four things registration can mean are kept apart here, which is the whole
/// point of returning a value instead of `Result<()>`:
///
/// - the process could not start - **fail**: nothing was registered, and only a
///   working CLI can fix that;
/// - it started and exited non-zero - **warn**: it has already explained itself on
///   stderr, and Hermes may still discover the plugin on its next scan;
/// - it exited zero - **the command succeeded**, which is all this step observed;
/// - whether Hermes then loads the plugin - **not verified here**, and said so in
///   the output rather than implied by a success message.
///
/// The presentation lives here rather than in `Register::Fn` so that the outcome is a
/// fact and the wording is a decision this layer owns.
pub(crate) fn Fn() -> anyhow::Result<Register::Outcome::Enum::Enum> {
	let Outcome = Register::Fn::Fn();

	match Outcome.clone() {
		Register::Outcome::Enum::Enum::NotStarted { Cause } => {
			anyhow::bail!("run `hermes plugins enable aphrodite`: {Cause}")
		},
		Register::Outcome::Enum::Enum::NonZero { Stderr, Code } => {
			let Status = Code.map_or_else(|| "killed by a signal".to_string(), |Code| format!("exit {Code}"));

			crate::Logging::Stderr::Fn::Fn(&format!("warning: hermes plugins enable aphrodite ({Status}): {Stderr}"));
		},
		Register::Outcome::Enum::Enum::Zero => {
			println!("plugin registered with hermes");
			println!("  note: setup does not read Hermes' plugin list back - whether the plugin loads is unverified");
		},
	}

	Ok(Outcome)
}
