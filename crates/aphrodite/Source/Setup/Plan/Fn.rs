//! Deciding what a setup run will do.

use anyhow::Context;

use crate::{
	Configuration::CLI::Setup::Struct::Struct as SetupArgs,
	Home,
	Setup::{
		Plan::{Step, Struct},
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
/// The two conditions that shape the run are decided here rather than inside the
/// executor, because they are properties of the machine, not of a step:
///
/// - a **catalog install** - a real directory under the Hermes home, placed there by
///   `download.sh`/`download.ps1` - is Hermes-owned and report-only; setup must not
///   write into it or delete it;
/// - a **stale symlink** from an older install (plugin dir -> runtime home) is
///   removed so the loader files land as real files, never through the link.
///
/// The step list is then that decision: `WriteManifest`/`WriteLoader` are simply
/// absent for a catalog install, and `WriteConfiguration` is absent when the file
/// already exists and `--force` was not given.
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

	// Absent dir (fresh setup) or dangling symlink (stale dev link): nothing of a
	// catalog install exists, so this is the dev path.
	let CatalogInstall = std::fs::canonicalize(&PluginDirectory)
		.map(|Resolved| Resolved.starts_with(std::fs::canonicalize(&HermesHome).unwrap_or_else(|_| HermesHome.clone())))
		.unwrap_or(false);

	let RemoveStaleSymlink = !CatalogInstall
		&& PluginDirectory
			.symlink_metadata()
			.map(|m| m.file_type().is_symlink())
			.unwrap_or(false);

	let ConfigurationPath = Context.AphroditeDirectory.join("aphrodite.toml");
	let WriteConfiguration = !ConfigurationPath.exists() || Args.Force;

	let mut Steps = vec![
		Step::Enum::Enum::CreateDirectories,
		Step::Enum::Enum::PreparePluginDirectory,
		Step::Enum::Enum::InstallBinary,
		Step::Enum::Enum::InstallDylibs,
	];

	let _ = WriteConfiguration.then(|| Steps.push(Step::Enum::Enum::WriteConfiguration));

	// The loader and the manifest are the only two artifacts that go into the
	// Hermes home, so they are the only two a catalog install makes report-only.
	let _ = (!CatalogInstall).then(|| {
		Steps.push(Step::Enum::Enum::WriteManifest);
		Steps.push(Step::Enum::Enum::WriteLoader);
	});

	Steps.push(Step::Enum::Enum::WriteVersionPin);
	Steps.push(Step::Enum::Enum::RegisterPlugin);

	Ok(Struct::Struct { Context, CatalogInstall, RemoveStaleSymlink, Steps })
}
