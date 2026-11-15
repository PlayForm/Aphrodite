//! Registering the installed plugin with Hermes.
//!
//! The last step of setup: `hermes plugins enable aphrodite`, so Hermes picks the
//! loader up on its next start. Everything the plugin needs is already on disk by
//! the time this runs, which is what makes a failure here a registration problem
//! rather than an install problem.

use std::process::Command;

use crate::Setup::Run::Context::Struct as SetupContext;

/// Enable the `Aphrodite` plugin through the `hermes` CLI.
///
/// A spawn failure is an error whose context names the command and fails the run;
/// a non-zero exit only warns. The asymmetry is deliberate: if the CLI cannot be
/// started nothing was registered, while a CLI that ran and refused has already
/// told the user why on stderr, and the plugin may still be discovered on Hermes'
/// next scan.
///
/// It is not verification - setup never reads back Hermes' plugin list to confirm
/// the enable took effect - and the context argument is unused, kept so the
/// signature matches the other write steps. Reporting success from this function
/// means the command exited zero, not that Hermes will load the plugin.
pub(crate) fn Fn(_Context:&SetupContext) -> anyhow::Result<()> {
	let Status = Command::new("hermes")
		.args(["plugins", "enable", "aphrodite"])
		.output()
		.map_err(|e| anyhow::Error::new(e).context("run `hermes plugins enable aphrodite`"))?;

	match !Status.status.success() {
		true => {
			let Stderr = String::from_utf8_lossy(&Status.stderr);
			eprintln!("warning: hermes plugins enable aphrodite: {Stderr}");
		},
		false => {
			println!("plugin registered with hermes");
		},
	}
	Ok(())
}
