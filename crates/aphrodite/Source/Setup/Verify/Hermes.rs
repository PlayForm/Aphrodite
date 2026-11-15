//! The `hermes` CLI prerequisite check.
//!
//! Setup registers the plugin by shelling out to `hermes plugins enable`, so the
//! CLI has to exist and answer before any file is written. Checking first is what
//! keeps a missing Hermes from leaving an installed binary, config and loader
//! behind with no registration.

use std::process::Command;

/// Verify the `hermes` CLI is on `PATH` and answers `--version`.
///
/// Prints the version it found, which is the only evidence the user gets that the
/// right Hermes was picked up. A non-zero exit and a spawn failure are both an
/// error whose message carries the command's own stderr or the spawn cause, so the
/// difference between "not installed" and "installed but broken" survives.
///
/// It is not a version gate: the plugin manifest states `requires_hermes`, and
/// enforcing it here would duplicate that check in a place the user cannot see.
/// It also does not look for `hermes` outside `PATH` - an install that only works
/// with an absolute path is reported as missing, which is the honest answer for
/// every later step that shells out the same way.
pub(crate) fn Fn() -> anyhow::Result<()> {
	match Command::new("hermes").arg("--version").output() {
		Ok(Out) if Out.status.success() => {
			let version = String::from_utf8_lossy(&Out.stdout).trim().to_string();
			println!("  hermes found: {version}");
			Ok(())
		},
		Ok(Out) => {
			let Stderr = String::from_utf8_lossy(&Out.stderr);
			anyhow::bail!("hermes --version failed: {Stderr}")
		},
		Err(e) => Err(anyhow::Error::new(e).context("hermes not found in PATH - install hermes agent first")),
	}
}
