//! Turning the configuration into the listener table.

use aphrodite::Configuration::CLI::Struct::Struct as CLI;

use super::{
	super::{Credential, Source},
	Assemble,
};

/// Build the listener table from the config, or from the command line.
///
/// Returns `(name, cli)` per listener. Fails on a `[[proxies]]` entry that
/// cannot resolve, and - for the standalone path - on a missing API key, so a
/// proxy never starts forwarding an empty `Bearer`.
///
/// The two arms are the two configuration sources, so this is the one place the
/// file and the command line are told apart. Nothing else in startup branches on
/// which one is in force.
pub(crate) fn Fn(Source:Source::Enum::Enum) -> anyhow::Result<Vec<(String, CLI)>> {
	match Source {
		Source::Enum::Enum::Multi(Configuration) => Assemble::Fn::Fn(&Configuration),
		Source::Enum::Enum::Standalone(CLI) => {
			Credential::Fn::Fn(&CLI)?;

			let Name = format!("{}", CLI.Listen);

			Ok(vec![(Name, CLI)])
		},
	}
}
