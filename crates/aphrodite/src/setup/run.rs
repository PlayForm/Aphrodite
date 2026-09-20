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
	pub(crate) own_path:PathBuf,
	pub(crate) own_hash:String,
}

/// Run the setup/bootstrap process.
pub fn run(args:&SetupArgs) -> Result<(), SetupError> {
	let home =
		dirs::home_dir().ok_or_else(|| SetupError::Io(io::Error::new(io::ErrorKind::NotFound, "$HOME not set")))?;

	let own_path = std::env::current_exe().map_err(SetupError::Io)?;
	let own_hash = self_hash(&own_path);

	let ctx = SetupCtx {
		aphrodite_dir:home.join(".hermes").join("aphrodite"),
		binaries_dir:home.join(".hermes").join("aphrodite").join("binaries"),
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
	write_plugin_yaml(&ctx, args)?;

	// ── Step 8: Write __init__.py shim ──
	write_init_py(&ctx)?;

	// ── Step 9: Register with hermes ──
	register_plugin(&ctx)?;

	println!("aphrodite installed -> {}", ctx.aphrodite_dir.display());
	println!(
		"plugin directory ready: {} (setup no longer symlinks it into Hermes automatically)",
		ctx.aphrodite_dir.display()
	);
	println!(
		"link it manually: ln -s {} {}/plugins/aphrodite",
		ctx.aphrodite_dir.display(),
		home.display()
	);

	Ok(())
}
