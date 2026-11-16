//! The `setup` subcommand, when the second argument asks for it.

use aphrodite::Configuration::CLI::{
	Command::Enum::Enum as Command,
	Setup::Struct::Struct as SetupArgs,
	Struct::Struct as CLI,
};
use clap::Parser;

use super::super::Enum::Enum;

/// The `setup` request, or `None` when the second argument is something else -
/// or when `setup` was named but clap resolved no `Setup` command, in which case
/// the original behaviour is to fall through and start the proxies.
///
/// `CLI::parse` runs only inside the branch that needs it, and it prints help or
/// a usage error and exits the process itself, so this function never has to
/// describe a parse failure.
pub(crate) fn Fn(Second:Option<&str>) -> Option<Enum> {
	(Second == Some("setup")).then(CLI::parse).and_then(|CLI:CLI| {
		match CLI.Command {
			Some(Command::Setup { Key, URL, Model, Cache, Token, NoLaunch, Force }) => {
				Some(Enum::Setup(SetupArgs { Key, URL, Model, Cache, Token, NoLaunch, Force }))
			},
			Some(Command::Run) | None => None,
		}
	})
}
