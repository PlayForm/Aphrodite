//! What an unreachable dylib means for the install.

use super::super::Requirement;

/// Fail or warn, according to how much the dylib matters.
///
/// `Required` fails with the two things the operator can actually do - build from
/// source, or fetch the asset by hand - because "download failed" on its own
/// leaves them with no next step. `Optional` warns and lets the install finish:
/// the Hermes plugin loads only the `*_hermes` dylib, so a missing core library
/// changes nothing about whether the plugin works.
pub(crate) fn Fn(Requirement:Requirement::Enum::Enum, Name:&str, Cause:anyhow::Error) -> anyhow::Result<()> {
	match Requirement {
		Requirement::Enum::Enum::Required => Err(Cause.context(format!(
			"dylib '{Name}' not found locally and download failed.  Build from source (cargo build --release -p aphrodite -p aphrodite-hermes) or download manually from https://github.com/PlayForm/Aphrodite/releases/tag/Aphrodite/v{}",
			env!("CARGO_PKG_VERSION")
		))),
		Requirement::Enum::Enum::Optional => {
			crate::Logging::Stderr::Fn::Fn(&format!(
				"WARNING: optional dylib '{Name}' could not be located or downloaded ({Cause}); continuing without it - the Hermes plugin loads only the *_hermes dylib"
			));

			Ok(())
		},
	}
}
