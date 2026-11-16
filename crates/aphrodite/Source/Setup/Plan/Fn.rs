//! Deciding what a setup run will do.

use anyhow::Context;

use crate::{
	Configuration::CLI::Setup::Struct::Struct as SetupArgs,
	Home,
	Setup::{
		Plan::{Ownership, Step, Struct},
		Run::Context::Struct as SetupContext,
		Verify::SelfHash::Fn as SelfHash,
	},
};

/// Resolve the install plan, writing nothing.
///
/// Two homes, resolved once and threaded through every step: the Hermes home, where
/// the plugin loader lives, and the runtime home, where binaries, configuration and
/// state live. Deriving them twice is how an install ends up with its loader in one
/// home and its dylib in another. Under a non-default Hermes home (Docker, profile
/// gateways) the runtime home is `<hermes-home>/aphrodite`, never a second
/// `~/.hermes/aphrodite` (issue 40).
///
/// The plugin directory's ownership is decided here rather than inside the executor,
/// because it is a property of the machine, not of a step: a **catalog install** - a
/// real directory under the Hermes home, placed there by
/// `download.sh`/`download.ps1` - is Hermes-owned and report-only, a **stale symlink**
/// from an older install is removed so the loader lands as real files, and a path that
/// exists but cannot be inspected is report-only because writing into it is the one
/// mistake setup must not make. `Ownership::Fn` is that decision; this function only
/// reads it.
///
/// The step list is then that decision: `WriteManifest`/`WriteLoader` are simply
/// absent for a directory setup does not own, and `WriteConfiguration` is absent when
/// the file already exists and `--force` was not given.
pub(crate) fn Fn(Args:&SetupArgs) -> anyhow::Result<Struct::Struct> {
	let HermesHome = Home::Hermes::Opt::Fn::Fn().context("no Hermes home resolvable (set HERMES_HOME or HOME)")?;
	let RuntimeHome = Home::Runtime::Opt::Fn::Fn()
		.context("no runtime home resolvable (set APHRODITE_HOME, HERMES_HOME, or HOME)")?;
	let OwnPath = std::env::current_exe().context("resolve the path of this executable")?;
	let OwnHash = SelfHash(&OwnPath);
	let PluginDirectory = HermesHome.join("plugins").join("aphrodite");

	let Context = SetupContext {
		AphroditeDirectory:RuntimeHome.clone(),
		BinariesDirectory:RuntimeHome.join("binaries"),
		PluginDirectory:PluginDirectory.clone(),
		OwnPath,
		OwnHash,
	};

	let Ownership = Ownership::Fn::Fn(&PluginDirectory, &HermesHome);

	let SetupOwnsDirectory = matches!(Ownership, Ownership::Enum::Enum::Dev | Ownership::Enum::Enum::DevStaleSymlink);

	let ConfigurationPath = Context.AphroditeDirectory.join("aphrodite.toml");
	let WriteConfiguration = !ConfigurationPath.exists() || Args.Force;

	let Steps:Vec<Step::Enum::Enum> = vec![
		Step::Enum::Enum::CreateDirectories,
		Step::Enum::Enum::PreparePluginDirectory,
		Step::Enum::Enum::InstallBinary,
		Step::Enum::Enum::InstallDylibs,
	]
	.into_iter()
	.chain(WriteConfiguration.then_some(Step::Enum::Enum::WriteConfiguration))
	// The loader and the manifest are the only two artifacts that go into the Hermes
	// home, so they are the only two a directory setup does not own makes report-only.
	.chain(SetupOwnsDirectory.then_some(Step::Enum::Enum::WriteManifest))
	.chain(SetupOwnsDirectory.then_some(Step::Enum::Enum::WriteLoader))
	.chain([Step::Enum::Enum::WriteVersionPin, Step::Enum::Enum::RegisterPlugin])
	.collect();

	Ok(Struct::Struct { Context, Ownership, Steps })
}
