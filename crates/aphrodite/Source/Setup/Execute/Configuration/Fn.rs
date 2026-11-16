//! Writing `aphrodite.toml` from the embedded template.

use std::fs;

use anyhow::Context;

use crate::{
	Configuration::CLI::Setup::Struct::Struct as SetupArgs,
	Setup::{
		Run::Context::Struct as SetupContext,
		Write::{ConfigurationTemplate::Const as ConfigurationTemplate, Permissions::Fn as SecurePerms},
	},
};

/// Write the configuration template, substituting the operator's values.
///
/// This is the only artifact setup preserves: the file carries the operator's ports
/// and upstream, so rewriting it on every run would silently discard their
/// configuration. `--force` is the explicit override, and the plan already left this
/// step out when neither condition holds.
///
/// Only the four placeholders the template declares are substituted. `api_url` and
/// `model` come from the command line and the environment, never from a write into
/// the document's other keys.
pub(crate) fn Fn(Context:&SetupContext, Args:&SetupArgs) -> anyhow::Result<()> {
	let ConfigurationPath = Context.AphroditeDirectory.join("aphrodite.toml");

	let Configuration = ConfigurationTemplate
		.replace("{api_url}", &Args.URL)
		.replace("{model}", &Args.Model)
		.replace("{cache_port}", &Args.Cache.to_string())
		.replace("{token_port}", &Args.Token.to_string());

	println!("writing config -> {}", ConfigurationPath.display());

	fs::write(&ConfigurationPath, &Configuration)
		.with_context(|| format!("write the configuration {}", ConfigurationPath.display()))?;

	SecurePerms(&ConfigurationPath, 0o600)?;

	Ok(())
}
