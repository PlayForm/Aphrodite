use std::{fs, io, path::PathBuf};

use super::*;
use crate::config::SetupArgs;

/// Errors that can occur during setup.
#[derive(Debug, thiserror::Error)]
pub enum SetupError {
	#[error("I/O error: {0}")]
	Io(#[from] io::Error),
	#[error("{0}")]
	HermesNotFound(String),
	#[error("{0}")]
	DylibNotFound(String),
	#[error("{0}")]
	PluginRegistrationFailed(String),
}

/// Context gathered during setup.
pub(crate) struct SetupCtx {
	pub(crate) aphrodite_dir:PathBuf,
	pub(crate) binaries_dir:PathBuf,
	pub(crate) plugin_dir:PathBuf,
	pub(crate) own_path:PathBuf,
	pub(crate) own_hash:String,
}

/// Run the setup/bootstrap process.
pub fn run(args:&SetupArgs) -> Result<(), SetupError> {
	let hermes_home = crate::home::hermes_home_opt().ok_or_else(|| {
		SetupError::Io(io::Error::new(
			io::ErrorKind::NotFound,
			"no Hermes home resolvable (set HERMES_HOME or HOME)",
		))
	})?;
	// Runtime home is a single shared decision (home::runtime_home:
	// $APHRODITE_HOME -> $HERMES_HOME -> $HOME -> platform default) so
	// `aphrodite setup` bootstraps exactly the home the plugin shim and the
	// running binary will use - under a non-default Hermes home (Docker,
	// profile gateways) that is <hermes-home>/aphrodite, never a second
	// ~/.hermes/aphrodite (issue 40).
	let runtime_home = crate::home::runtime_home_opt().ok_or_else(|| {
		SetupError::Io(io::Error::new(
			io::ErrorKind::NotFound,
			"no runtime home resolvable (set APHRODITE_HOME, HERMES_HOME, or HOME)",
		))
	})?;

	let own_path = std::env::current_exe().map_err(SetupError::Io)?;
	let own_hash = self_hash(&own_path);

	let ctx = SetupCtx {
		aphrodite_dir:runtime_home.clone(),
		binaries_dir:runtime_home.join("binaries"),
		plugin_dir:hermes_home.join("plugins").join("aphrodite"),
		own_path,
		own_hash,
	};

	println!("aphrodite setup v{}", env!("CARGO_PKG_VERSION"));
	println!("   self-hash: {}", ctx.own_hash);

	// ── Step 1: Check prerequisites ──
	verify_hermes()?;

	// ── Step 2: Create directory structure ──
	fs::create_dir_all(&ctx.binaries_dir)?;
	fs::create_dir_all(&ctx.aphrodite_dir)?;

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
	let plugin_is_catalog_install = match fs::canonicalize(&ctx.plugin_dir) {
		Ok(resolved) => {
			let home = fs::canonicalize(&hermes_home).unwrap_or_else(|_| hermes_home.clone());
			resolved.starts_with(&home)
		},
		// Absent dir (fresh setup) or dangling symlink (stale dev link):
		// nothing of a catalog install exists - dev path.
		Err(_) => false,
	};
	if plugin_is_catalog_install {
		// Report-only: never remove symlinks, never create or write the
		// plugin dir. A catalog install must be updated via
		// download.sh/download.ps1, not `aphrodite setup`, and setup never
		// deletes the plugin dir.
		println!(
			"NOTE: {} is the plugin's own install dir under the Hermes home (catalog install) - report-only, left \
			 untouched",
			ctx.plugin_dir.display()
		);
		println!("  a catalog install must use `bash download.sh` / `pwsh download.ps1`, not `aphrodite setup`");
		println!("  `aphrodite setup` will not remove, create, or overwrite files there");
	} else {
		if ctx
			.plugin_dir
			.symlink_metadata()
			.map(|m| m.file_type().is_symlink())
			.unwrap_or(false)
		{
			println!("removing stale plugin symlink -> {}", ctx.plugin_dir.display());
			fs::remove_file(&ctx.plugin_dir)?;
		}
		fs::create_dir_all(&ctx.plugin_dir)?;
	}

	// ── Step 3: Copy self to binaries dir (always overwrite - the binary
	// is the install payload; config is preserved unless --force) ──
	let target_binary = ctx.binaries_dir.join(binary_name());
	println!("copying binary -> {}", target_binary.display());
	// macOS: `fs::copy` preserves extended attributes (code signature,
	// quarantine) from the build directory - Gatekeeper kills the copied
	// binary at the install path. See `install_macos_artifact`'s doc
	// comment (03-F6/F7/F9) for why this isn't just `ditto` + `let _ =`.
	#[cfg(target_os = "macos")]
	install_macos_artifact(&ctx.own_path, &target_binary, None, 0o700)?;
	#[cfg(not(target_os = "macos"))]
	{
		fs::copy(&ctx.own_path, &target_binary)?;
		secure_perms(&target_binary, 0o700)?;
	}

	// ── Step 5: Find and copy dylibs ──
	copy_dylibs(&ctx)?;

	// ── Step 6: Write aphrodite.toml from template ──
	let config_path = ctx.aphrodite_dir.join("aphrodite.toml");
	if !config_path.exists() || args.force {
		let config = CONFIG_TEMPLATE
			.replace("{api_url}", &args.api_url)
			.replace("{model}", &args.model)
			.replace("{cache_port}", &args.cache_port.to_string())
			.replace("{token_port}", &args.token_port.to_string());
		println!("writing config -> {}", config_path.display());
		fs::write(&config_path, &config)?;
		secure_perms(&config_path, 0o600)?;
	}

	// ── Step 7: Write plugin.yaml ──
	// ── Step 8: Write __init__.py shim ──
	// Both write the loader INTO the plugin dir - skipped in catalog-install
	// mode (report-only, PR 118488); the Hermes-home plugin dir is the
	// plugin's own install dir there and setup must not touch it.
	if !plugin_is_catalog_install {
		write_plugin_yaml(&ctx, args)?;
		write_init_py(&ctx)?;
	}

	// ── Step 8b: Write the BINARY_VERSION pin into the runtime home ──
	write_binary_version(&ctx)?;

	// ── Step 9: Register with hermes ──
	register_plugin(&ctx)?;

	println!("aphrodite installed -> {}", ctx.aphrodite_dir.display());
	println!(
		"  binaries, config, and state: {} (everything the plugin manages)",
		ctx.aphrodite_dir.display()
	);
	if plugin_is_catalog_install {
		println!(
			"  plugin dir (catalog install, report-only): {} - loader NOT written by setup; use \
			 download.sh/download.ps1",
			ctx.plugin_dir.display()
		);
	} else {
		println!("  hooks registered (loader only): {}", ctx.plugin_dir.display());
	}

	Ok(())
}
