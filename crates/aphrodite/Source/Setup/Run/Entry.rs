//! The setup entry point.
//!
//! One function owns the whole install order, because the order is the contract:
//! resolve both homes, verify prerequisites, create the directories, copy the
//! binary, install the dylibs, write configuration, write the loader, pin the
//! version, register the plugin. Each step depends on the previous one having
//! succeeded, and a step that runs early leaves an installed layout the later
//! steps never completed.

use std::fs;

use anyhow::Context;

use crate::{
	Configuration::CLI::Setup::Struct::Struct as SetupArgs,
	Setup::{
		Dylib::Copy::Fn as CopyDylibs,
		Run::Context::Struct as SetupContext,
		Verify::{Hermes::Fn as VerifyHermes, SelfHash::Fn as SelfHash},
		Write::{
			BinaryName::Fn as BinaryName,
			BinaryVersion::Fn as WriteBinaryVersion,
			ConfigurationTemplate::Const as ConfigurationTemplate,
			InitPy::Fn as WriteInitPy,
			Permissions::Fn as SecurePerms,
			PluginYAML::Fn as WritePluginYAML,
			Register::Fn as RegisterPlugin,
		},
	},
};
#[cfg(target_os = "macos")]
use crate::Setup::MacOS::Fn as InstallMacOSArtifact;

/// Run the setup/bootstrap process.
///
/// Resolves the Hermes home and the runtime home, then installs everything the
/// plugin needs: the binary and dylibs into `<runtime-home>/binaries/`, the
/// configuration template into `<runtime-home>/aphrodite.toml`, and the
/// hooks-only loader into `<hermes-home>/plugins/aphrodite/`. A catalog install -
/// a real plugin directory under the Hermes home, placed there by
/// `download.sh`/`download.ps1` - is report-only: setup prints what it would
/// have done and touches nothing in it.
///
/// It is not idempotent in the "only what changed" sense and it is not a repair
/// tool: the binary, the dylibs, the manifest, the loader and the version pin are
/// always rewritten, only `aphrodite.toml` is preserved unless `--force`, and a
/// run that fails halfway leaves the artifacts it already wrote. Re-running is
/// the repair.
///
/// It is also not safe to call twice concurrently, and it must resolve the same
/// homes the plugin will: under a non-default Hermes home (Docker, profile
/// gateways) setup writing to a second `~/.hermes/aphrodite` produces an install
/// that looks complete and never loads.
pub fn Fn(Args:&SetupArgs) -> anyhow::Result<()> {
	let HermesHome =
		crate::Home::Hermes::Opt::Fn::Fn().context("no Hermes home resolvable (set HERMES_HOME or HOME)")?;
	// Runtime home is a single shared decision (Home::Runtime::Fn::Fn:
	// $APHRODITE_HOME -> $HERMES_HOME -> $HOME -> platform default) so
	// `aphrodite setup` bootstraps exactly the home the plugin shim and the
	// running binary will use - under a non-default Hermes home (Docker,
	// profile gateways) that is <hermes-home>/aphrodite, never a second
	// ~/.hermes/aphrodite (issue 40).
	let RuntimeHome = crate::Home::Runtime::Opt::Fn::Fn()
		.context("no runtime home resolvable (set APHRODITE_HOME, HERMES_HOME, or HOME)")?;

	let OwnPath = std::env::current_exe().context("resolve the path of this executable")?;
	let OwnHash = SelfHash(&OwnPath);

	let Context = SetupContext {
		AphroditeDirectory:RuntimeHome.clone(),
		BinariesDirectory:RuntimeHome.join("binaries"),
		PluginDirectory:HermesHome.join("plugins").join("aphrodite"),
		OwnPath,
		OwnHash,
	};

	println!("aphrodite setup v{}", env!("CARGO_PKG_VERSION"));
	println!("   self-hash: {}", Context.OwnHash);

	// ── Step 1: Check prerequisites ──
	VerifyHermes()?;

	// ── Step 2: Create directory structure ──
	fs::create_dir_all(&Context.BinariesDirectory)
		.with_context(|| format!("create the binaries directory {}", Context.BinariesDirectory.display()))?;
	fs::create_dir_all(&Context.AphroditeDirectory)
		.with_context(|| format!("create the runtime home {}", Context.AphroditeDirectory.display()))?;

	// ── Step 2b: Prepare the Hermes plugin dir (hooks-only) ──
	// The plugin dir holds ONLY the loader (plugin.yaml + __init__.py) that
	// registers hooks/tools; every runtime artifact (binaries, config, state)
	// lives in the runtime home. A stale symlink from older installs (plugin
	// dir -> runtime home) is removed first so the loader files land as real
	// files, never through the link into the runtime home.
	//
	// Catalog-install guard (PR 118488 review, teknium1): when
	// <hermes-home>/plugins/aphrodite is the plugin's OWN install dir - a
	// real directory under the Hermes home placed there by a catalog install
	// (download.sh/download.ps1) - `aphrodite setup` must never write into
	// it or delete it: that path is Hermes-owned and REPORT-ONLY. The dev
	// path (plugin dir absent, or a symlink resolving OUTSIDE the Hermes
	// home - an old dev link into the source checkout) keeps the historical
	// behavior below unchanged.
	let PluginIsCatalogInstall = match fs::canonicalize(&Context.PluginDirectory) {
		Ok(Resolved) => {
			let Home = fs::canonicalize(&HermesHome).unwrap_or_else(|_| HermesHome.clone());
			Resolved.starts_with(&Home)
		},
		// Absent dir (fresh setup) or dangling symlink (stale dev link):
		// nothing of a catalog install exists - dev path.
		Err(_) => false,
	};

	match PluginIsCatalogInstall {
		true => {
			// Report-only: never remove symlinks, never create or write the
			// plugin dir. A catalog install must be updated via
			// download.sh/download.ps1, not `aphrodite setup`, and setup never
			// deletes the plugin dir.
			println!(
				"NOTE: {} is the plugin's own install dir under the Hermes home (catalog install) - report-only, left \
				 untouched",
				Context.PluginDirectory.display()
			);
			println!("  a catalog install must use `bash download.sh` / `pwsh download.ps1`, not `aphrodite setup`");
			println!("  `aphrodite setup` will not remove, create, or overwrite files there");
		},
		false => {
			match Context
				.PluginDirectory
				.symlink_metadata()
				.map(|m| m.file_type().is_symlink())
				.unwrap_or(false)
			{
				true => {
					println!("removing stale plugin symlink -> {}", Context.PluginDirectory.display());
					fs::remove_file(&Context.PluginDirectory).with_context(|| {
						format!("remove the stale plugin symlink {}", Context.PluginDirectory.display())
					})?;
				},
				false => (),
			}
			fs::create_dir_all(&Context.PluginDirectory)
				.with_context(|| format!("create the plugin directory {}", Context.PluginDirectory.display()))?;
		},
	}

	// ── Step 3: Copy self to binaries dir (always overwrite - the binary
	// is the install payload; config is preserved unless --force) ──
	let TargetBinary = Context.BinariesDirectory.join(BinaryName());
	println!("copying binary -> {}", TargetBinary.display());
	// macOS: `fs::copy` preserves extended attributes (code signature,
	// quarantine) from the build directory - Gatekeeper kills the copied
	// binary at the install path. See `MacOS::Fn`'s doc comment
	// (03-F6/F7/F9) for why this isn't just `ditto` + `let _ =`.
	#[cfg(target_os = "macos")]
	InstallMacOSArtifact(&Context.OwnPath, &TargetBinary, None, 0o700)?;
	#[cfg(not(target_os = "macos"))]
	{
		fs::copy(&Context.OwnPath, &TargetBinary)
			.with_context(|| format!("copy this executable to {}", TargetBinary.display()))?;
		SecurePerms(&TargetBinary, 0o700)?;
	}

	// ── Step 5: Find and copy dylibs ──
	CopyDylibs(&Context)?;

	// ── Step 6: Write aphrodite.toml from template ──
	let ConfigurationPath = Context.AphroditeDirectory.join("aphrodite.toml");

	match !ConfigurationPath.exists() || Args.Force {
		true => {
			let Configuration = ConfigurationTemplate
				.replace("{api_url}", &Args.APIURL)
				.replace("{model}", &Args.Model)
				.replace("{cache_port}", &Args.CachePort.to_string())
				.replace("{token_port}", &Args.TokenPort.to_string());
			println!("writing config -> {}", ConfigurationPath.display());
			fs::write(&ConfigurationPath, &Configuration)
				.with_context(|| format!("write the configuration {}", ConfigurationPath.display()))?;
			SecurePerms(&ConfigurationPath, 0o600)?;
		},
		false => (),
	}

	// ── Step 7: Write plugin.yaml ──
	// ── Step 8: Write __init__.py shim ──
	// Both write the loader INTO the plugin dir - skipped in catalog-install
	// mode (report-only, PR 118488); the Hermes-home plugin dir is the
	// plugin's own install dir there and setup must not touch it.
	match !PluginIsCatalogInstall {
		true => {
			WritePluginYAML(&Context, Args)?;
			WriteInitPy(&Context)?;
		},
		false => (),
	}

	// ── Step 8b: Write the BINARY_VERSION pin into the runtime home ──
	WriteBinaryVersion(&Context)?;

	// ── Step 9: Register with hermes ──
	RegisterPlugin(&Context)?;

	println!("aphrodite installed -> {}", Context.AphroditeDirectory.display());
	println!(
		"  binaries, config, and state: {} (everything the plugin manages)",
		Context.AphroditeDirectory.display()
	);

	match PluginIsCatalogInstall {
		true => {
			println!(
				"  plugin dir (catalog install, report-only): {} - loader NOT written by setup; use \
				 download.sh/download.ps1",
				Context.PluginDirectory.display()
			);
		},
		false => {
			println!("  hooks registered (loader only): {}", Context.PluginDirectory.display());
		},
	}

	Ok(())
}
