//! Bootstrap setup: `aphrodite setup` - one-shot install after `cargo install aphrodite`.
//!
//! Creates ~/.hermes/aphrodite/ with binaries, config, and plugin manifest,
//! then optionally launches the proxy. Maximum security: self-verification,
//! permission hardening, no secrets in args.
//!
//! Templates live in `templates/` and are embedded at compile time via
//! `include_str!` - no runtime file dependency for cargo-installed binaries.

use std::{
	fs,
	io,
	path::{Path, PathBuf},
	process::Command,
};

use crate::config::SetupArgs;

/// aphrodite.toml template - embedded at compile time.
/// Placeholders: `{api_url}`, `{model}`, `{cache_port}`, `{token_port}` -
/// replaced with user-provided values.
const CONFIG_TEMPLATE: &str = include_str!("../templates/aphrodite.toml");

/// Errors that can occur during setup.
#[derive(Debug)]
pub enum SetupError {
	Io(io::Error),
	HermesNotFound(String),
	AlreadyInstalled,
	DylibNotFound(String),
	PluginRegistrationFailed(String),
}

impl std::fmt::Display for SetupError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Io(e) => write!(f, "I/O error: {e}"),
			Self::HermesNotFound(msg) => write!(f, "{msg}"),
			Self::AlreadyInstalled => write!(f, "aphrodite already installed - use --force to re-setup"),
			Self::DylibNotFound(msg) => write!(f, "{msg}"),
			Self::PluginRegistrationFailed(msg) => write!(f, "{msg}"),
		}
	}
}

impl From<io::Error> for SetupError {
	fn from(e: io::Error) -> Self { Self::Io(e) }
}

impl std::error::Error for SetupError {}

/// Context gathered during setup.
struct SetupCtx {
	aphrodite_dir: PathBuf,
	binaries_dir: PathBuf,
	own_path: PathBuf,
	own_hash: String,
}

/// Run the setup/bootstrap process.
pub fn run(args: &SetupArgs) -> Result<(), SetupError> {
	let home = dirs::home_dir().ok_or_else(|| {
		SetupError::Io(io::Error::new(io::ErrorKind::NotFound, "$HOME not set"))
	})?;

	let own_path = std::env::current_exe().map_err(SetupError::Io)?;
	let own_hash = self_hash(&own_path);

	let ctx = SetupCtx {
		aphrodite_dir: home.join(".hermes").join("aphrodite"),
		binaries_dir: home.join(".hermes").join("aphrodite").join("binaries"),
		own_path,
		own_hash,
	};

	println!("aphrodite setup v{}", env!("CARGO_PKG_VERSION"));
	println!("   self-hash: {}", ctx.own_hash);

	// ── Step 1: Check prerequisites ──
	verify_hermes()?;

	// ── Step 2: Check if already installed ──
	let target_binary = ctx.binaries_dir.join(binary_name());
	if target_binary.exists() && !args.force {
		return Err(SetupError::AlreadyInstalled);
	}

	// ── Step 3: Create directory structure ──
	fs::create_dir_all(&ctx.binaries_dir)?;
	fs::create_dir_all(&ctx.aphrodite_dir)?;

	// ── Step 4: Copy self to binaries dir ──
	println!("copying binary -> {}", target_binary.display());
	fs::copy(&ctx.own_path, &target_binary)?;
	secure_perms(&target_binary, 0o700)?;

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

	// ── Step 9: Symlink to hermes plugins dir ──
	symlink_plugin(&ctx)?;

	// ── Step 10: Register with hermes ──
	register_plugin(&ctx)?;

	println!("aphrodite installed -> {}", ctx.aphrodite_dir.display());

	Ok(())
}

/// Compute BLAKE3 hash of the binary for integrity display.
fn self_hash(path: &Path) -> String {
	match fs::read(path) {
		Ok(bytes) => {
			let hash = blake3::hash(&bytes);
			hash.to_hex().to_string()
		}
		Err(_) => "unknown".into(),
	}
}

/// Verify hermes CLI is available.
fn verify_hermes() -> Result<(), SetupError> {
	match Command::new("hermes").arg("--version").output() {
		Ok(out) if out.status.success() => {
			let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
			println!("  hermes found: {version}");
			Ok(())
		},
		Ok(out) => {
			let stderr = String::from_utf8_lossy(&out.stderr);
			Err(SetupError::HermesNotFound(format!(
				"hermes --version failed: {stderr}"
			)))
		},
		Err(_) => Err(SetupError::HermesNotFound(
			"hermes not found in PATH - install hermes agent first".into()
		)),
	}
}

/// Copy dylibs from cargo build target to binaries dir.
fn copy_dylibs(ctx: &SetupCtx) -> Result<(), SetupError> {
	let dylib_names: &[&str] = if cfg!(target_os = "macos") {
		&["libaphrodite.dylib", "libaphrodite_hermes.dylib"]
	} else if cfg!(target_os = "linux") {
		&["libaphrodite.so", "libaphrodite_hermes.so"]
	} else {
		&["aphrodite.dll", "aphrodite_hermes.dll"]
	};

	let exe_dir = ctx.own_path.parent().unwrap_or(Path::new("."));
	let search_paths: Vec<PathBuf> = vec![
		exe_dir.to_path_buf(),
		exe_dir.join("deps"),
		PathBuf::from("/usr/local/lib"),
		PathBuf::from("/opt/homebrew/lib"),
	];

	let mut copied = 0u32;
	for name in dylib_names {
		let dest = ctx.binaries_dir.join(name);
		if dest.exists() {
			copied += 1;
			continue;
		}

		let mut found = false;
		for search_dir in &search_paths {
			let src = search_dir.join(name);
			if src.exists() {
				println!("copying dylib {} -> {}", name, dest.display());
				fs::copy(&src, &dest)?;
				secure_perms(&dest, 0o755)?;
				found = true;
				copied += 1;
				break;
			}
		}
		if !found {
			let target_release = exe_dir
				.parent()
				.unwrap_or(Path::new("."))
				.parent()
				.unwrap_or(Path::new("."))
				.join("target")
				.join("release")
				.join(name);
			if target_release.exists() {
				println!("copying dylib {} -> {}", name, dest.display());
				fs::copy(&target_release, &dest)?;
				secure_perms(&dest, 0o755)?;
				found = true;
				copied += 1;
			}
		}
		if !found {
			return Err(SetupError::DylibNotFound(format!(
				"dylib '{name}' not found. Build from source or download from GitHub Releases."
			)));
		}
	}

	println!("copied {copied} dylib(s)");
	Ok(())
}

/// Write plugin.yaml manifest.
fn write_plugin_yaml(ctx: &SetupCtx, args: &SetupArgs) -> Result<(), SetupError> {
	let path = ctx.aphrodite_dir.join("plugin.yaml");
	if path.exists() { return Ok(()); }

	let yaml = format!(
		r#"name: aphrodite
version: {version}
description: "CCR compression plugin - 12 tools, context engine, TOML-driven templates."
kind: standalone
min_hermes_version: "0.16.0"
requires_hooks: true
provides_hooks:
  - on_session_start
  - transform_tool_result
  - pre_llm_call
  - transform_terminal_output
  - post_llm_call
provides_tools:
  - aphrodite_retrieve
  - aphrodite_compress
  - aphrodite_stats
  - aphrodite_rebuild
  - aphrodite_files
  - aphrodite_diff
  - aphrodite_search
  - aphrodite_test
  - aphrodite_catalog
  - aphrodite_reclassify
  - aphrodite_prefetch
  - aphrodite_prefetch_status
provides_context_engine: true
install_message: |
  aphrodite v{version} - installed via `cargo install aphrodite` + `aphrodite setup`.
  All logic in binaries/ - Rust-powered. Secure defaults.
  Proxies: token (:{token_port}, SQLite), cache (:{cache_port}, in-memory).
"#,
		version = env!("CARGO_PKG_VERSION"),
		token_port = args.token_port,
		cache_port = args.cache_port,
	);
	println!("writing plugin manifest -> {}", path.display());
	fs::write(&path, &yaml)?;
	secure_perms(&path, 0o644)?;
	Ok(())
}

/// Write __init__.py shim for hermes plugin loading.
fn write_init_py(ctx: &SetupCtx) -> Result<(), SetupError> {
	let path = ctx.aphrodite_dir.join("__init__.py");
	if path.exists() { return Ok(()); }

	let shim = include_str!("../templates/__init__.py");
	println!("writing __init__.py -> {}", path.display());
	fs::write(&path, shim)?;
	secure_perms(&path, 0o644)?;
	Ok(())
}

/// Symlink ~/.hermes/plugins/aphrodite -> ~/.hermes/aphrodite/
fn symlink_plugin(ctx: &SetupCtx) -> Result<(), SetupError> {
	let plugins_dir = dirs::home_dir()
		.ok_or_else(|| SetupError::Io(io::Error::new(io::ErrorKind::NotFound, "$HOME not set")))?
		.join(".hermes")
		.join("plugins");
	fs::create_dir_all(&plugins_dir)?;
	let link = plugins_dir.join("aphrodite");

	if link.exists() {
		if link.is_symlink() {
			let target = fs::read_link(&link)?;
			if target == ctx.aphrodite_dir {
				return Ok(());
			}
			fs::remove_file(&link)?;
		} else {
			return Err(SetupError::PluginRegistrationFailed(format!(
				"{} exists and is not a symlink - manual cleanup required",
				link.display()
			)));
		}
	}

	#[cfg(unix)]
	std::os::unix::fs::symlink(&ctx.aphrodite_dir, &link)?;
	#[cfg(windows)]
	{
		// Real symlinks need elevated privileges on Windows; a directory
		// junction doesn't. Try that first (mirrors Maintain/install.bat),
		// falling back to a recursive copy if junctions are blocked too.
		let status = Command::new("cmd")
			.args(["/C", "mklink", "/J"])
			.arg(&link)
			.arg(&ctx.aphrodite_dir)
			.status();
		let junction_ok = matches!(status, Ok(s) if s.success());
		if !junction_ok {
			copy_dir_recursive(&ctx.aphrodite_dir, &link)?;
		}
	}
	#[cfg(not(any(unix, windows)))]
	{
		let _ = (&ctx.aphrodite_dir, &link);
	}
	println!("symlinked plugin -> {}", link.display());
	Ok(())
}

/// Recursively copy a directory tree - the Windows fallback when a junction
/// can't be created (e.g. `mklink` disabled by policy).
#[cfg(windows)]
fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
	fs::create_dir_all(dst)?;
	for entry in fs::read_dir(src)? {
		let entry = entry?;
		let dest_path = dst.join(entry.file_name());
		if entry.file_type()?.is_dir() {
			copy_dir_recursive(&entry.path(), &dest_path)?;
		} else {
			fs::copy(entry.path(), &dest_path)?;
		}
	}
	Ok(())
}

/// Register plugin with hermes.
fn register_plugin(_ctx: &SetupCtx) -> Result<(), SetupError> {
	let status = Command::new("hermes")
		.args(["plugins", "enable", "aphrodite"])
		.output()
		.map_err(|e| SetupError::PluginRegistrationFailed(format!("hermes plugins enable: {e}")))?;

	if !status.status.success() {
		let stderr = String::from_utf8_lossy(&status.stderr);
		eprintln!("warning: hermes plugins enable aphrodite: {stderr}");
	} else {
		println!("plugin registered with hermes");
	}
	Ok(())
}

/// Set strict file permissions (Unix only).
fn secure_perms(path: &Path, mode: u32) -> io::Result<()> {
	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		let mut perms = fs::metadata(path)?.permissions();
		perms.set_mode(mode);
		fs::set_permissions(path, perms)?;
	}
	#[cfg(not(unix))]
	let _ = (path, mode);
	Ok(())
}

fn binary_name() -> &'static str {
	if cfg!(target_os = "windows") { "aphrodite.exe" } else { "aphrodite" }
}
