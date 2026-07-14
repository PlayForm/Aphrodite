//! Bootstrap setup: `aphrodite setup` - one-shot install after
//! `cargo install aphrodite`.
//!
//! Creates ~/.hermes/aphrodite/ with binaries, config, and plugin manifest,
//! then optionally launches the proxy. Maximum security: self-verification,
//! permission hardening, no secrets in args.
//!
//! Templates live in `templates/` and are embedded at compile time via
//! `include_str!` - no runtime file dependency for cargo-installed binaries.

use std::{
	fs, io,
	path::{Path, PathBuf},
	process::Command,
};

use crate::config::SetupArgs;

/// aphrodite.toml template - embedded at compile time.
/// Placeholders: `{api_url}`, `{model}`, `{cache_port}`, `{token_port}` -
/// replaced with user-provided values.
const CONFIG_TEMPLATE: &str = include_str!("../templates/aphrodite.toml");

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
struct SetupCtx {
	aphrodite_dir: PathBuf,
	binaries_dir: PathBuf,
	own_path: PathBuf,
	own_hash: String,
}

/// Run the setup/bootstrap process.
pub fn run(args: &SetupArgs) -> Result<(), SetupError> {
	let home =
		dirs::home_dir().ok_or_else(|| SetupError::Io(io::Error::new(io::ErrorKind::NotFound, "$HOME not set")))?;

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

	// ── Step 9: Symlink to hermes plugins dir ──
	symlink_plugin(&ctx)?;

	// ── Step 10: Register with hermes ──
	register_plugin(&ctx)?;

	println!("aphrodite installed -> {}", ctx.aphrodite_dir.display());

	Ok(())
}

/// Copy `src` to `dest` on macOS with the full Gatekeeper-safe treatment,
/// then harden permissions. `dylib_id_name` is `Some(name)` for a dylib
/// copy - runs `install_name_tool -id @rpath/<name>` and an ad-hoc
/// `codesign` re-sign; `None` for a plain binary copy (no dylib ID to
/// rewrite, no signature-invalidating relink, so no re-sign needed).
///
/// 03-F6/F7/F9 - three related bugs this helper fixes at once by being the
/// single call site for every macOS artifact copy setup performs:
/// - **F6**: a *failed* (non-zero exit) `ditto` used to be treated as
///   success (`.status().is_err()` only catches spawn failure, not a bad
///   exit code), so the `fs::copy` fallback never ran - and since the
///   destination was `remove_file`'d moments earlier, the install could
///   proceed against a missing artifact.
/// - **F7**: the `target/release` dev-build fallback path (used when no
///   prebuilt dylib is found in the normal search paths) called a bare
///   `fs::copy` with none of this treatment - exactly the dev workflow the
///   original Gatekeeper fix (CHANGELOG v1.2.5) was written for, making the
///   SIGKILL bug look "intermittent" rather than "always broken for source
///   builds."
/// - **F9**: `install_name_tool`/`xattr` failures were silently swallowed
///   with `let _ = ...`, so a machine missing Xcode Command Line Tools got
///   the SIGKILL bug back with zero diagnostic signal. Ad-hoc re-signing is
///   now an explicit, warned-on-failure step too, instead of relying on
///   macOS to incidentally re-sign a linker-edited Mach-O.
#[cfg(target_os = "macos")]
fn install_macos_artifact(src: &Path, dest: &Path, dylib_id_name: Option<&str>, mode: u32) -> Result<(), SetupError> {
	let _ = std::fs::remove_file(dest);
	let ditto_ok = Command::new("ditto")
		.args([src.to_str().unwrap_or(""), dest.to_str().unwrap_or("")])
		.status()
		.map(|s| s.success())
		.unwrap_or(false);
	if !ditto_ok {
		fs::copy(src, dest)?;
		match Command::new("xattr").args(["-c", dest.to_str().unwrap_or("")]).output() {
			Ok(out) if out.status.success() => {},
			_ => {
				eprintln!(
					"warning: xattr -c failed or unavailable for {} - Gatekeeper may still kill this artifact",
					dest.display()
				)
			},
		}
	}
	if let Some(name) = dylib_id_name {
		let rpath = format!("@rpath/{name}");
		match Command::new("install_name_tool")
			.args(["-id", &rpath, dest.to_str().unwrap_or("")])
			.output()
		{
			Ok(out) if out.status.success() => {},
			_ => {
				eprintln!(
					"warning: install_name_tool failed or unavailable for {} - Gatekeeper may kill Hermes when \
					 loading this dylib; install Xcode Command Line Tools and re-run setup",
					dest.display()
				)
			},
		}
		// Ad-hoc re-sign: install_name_tool invalidates the dylib's embedded
		// signature on arm64.
		match Command::new("codesign")
			.args(["-f", "-s", "-", dest.to_str().unwrap_or("")])
			.output()
		{
			Ok(out) if out.status.success() => {},
			_ => {
				eprintln!(
					"warning: codesign failed or unavailable for {} - Gatekeeper may still kill this dylib",
					dest.display()
				)
			},
		}
	}
	secure_perms(dest, mode)?;
	Ok(())
}

/// Compute BLAKE3 hash of the binary for integrity display.
fn self_hash(path: &Path) -> String {
	match fs::read(path) {
		Ok(bytes) => {
			let hash = blake3::hash(&bytes);
			hash.to_hex().to_string()
		},
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
			Err(SetupError::HermesNotFound(format!("hermes --version failed: {stderr}")))
		},
		Err(_) => Err(SetupError::HermesNotFound(
			"hermes not found in PATH - install hermes agent first".into(),
		)),
	}
}

/// Copy dylibs from cargo build target to binaries dir.
///
/// Searches local paths first; falls back to downloading from the GitHub
/// release matching this binary's version so that `cargo install aphrodite
/// && aphrodite setup` works without a source checkout.
///
/// ## Release asset naming
///
/// `cargo build` produces:
///   - `target/release/libaphrodite.dylib`
///   - `target/release/libaphrodite_hermes.dylib`
///
/// GitHub Release assets are suffixed with the target triple:
///   - `libaphrodite-aarch64-apple-darwin.dylib`
///   - `libaphrodite_hermes-aarch64-apple-darwin.dylib`
///
/// The download path fetches the tripled name and saves it as the
/// un-tripled name the plugin expects.
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

		let mut found = false;
		for search_dir in &search_paths {
			let src = search_dir.join(name);
			if src.exists() {
				println!("copying dylib {} -> {}", name, dest.display());
				// Fix install name: `cargo build` embeds the target/deps/
				// path as the dylib's ID. Loading a copied dylib whose ID
				// points to a non-existent (or stale) build-directory path
				// causes the macOS dynamic linker to SIGKILL the process.
				// See `install_macos_artifact`'s doc comment (03-F6/F7/F9)
				// for why this isn't just `ditto`/`install_name_tool` +
				// `let _ =`.
				#[cfg(target_os = "macos")]
				install_macos_artifact(&src, &dest, Some(name), 0o755)?;
				#[cfg(not(target_os = "macos"))]
				{
					fs::copy(&src, &dest)?;
					secure_perms(&dest, 0o755)?;
				}
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
				// 03-F7: this fallback (dev builds where the dylib isn't
				// co-located with the setup binary) used to skip the
				// Gatekeeper treatment above entirely - same helper here too.
				#[cfg(target_os = "macos")]
				install_macos_artifact(&target_release, &dest, Some(name), 0o755)?;
				#[cfg(not(target_os = "macos"))]
				{
					fs::copy(&target_release, &dest)?;
					secure_perms(&dest, 0o755)?;
				}
				found = true;
				copied += 1;
			}
		}
		if !found {
			// Local search exhausted - try downloading from the GitHub
			// release matching this binary's version.  `cargo install`
			// only delivers [[bin]] targets, never cdylib artifacts, so
			// a `cargo install aphrodite && aphrodite setup` user will
			// always land here.  This download path bridges that gap
			// without requiring a full source checkout.
			if let Err(e) = download_dylib(name, &dest) {
				return Err(SetupError::DylibNotFound(format!(
					"dylib '{name}' not found locally and download failed: {e}.  \
					 Build from source (cargo build --release -p aphrodite -p aphrodite-hermes) \
					 or download manually from \
					 https://github.com/PlayForm/Aphrodite/releases/tag/Aphrodite/v{version}",
					version = env!("CARGO_PKG_VERSION"),
				)));
			}
			copied += 1;
		}
	}

	println!("copied {copied} dylib(s)");
	Ok(())
}

/// Download a single dylib from the GitHub release matching this binary's
/// version.  `dest_name` is the bare filename (e.g. `libaphrodite.dylib`);
/// the remote asset is named with a target-triple suffix (e.g.
/// `libaphrodite-aarch64-apple-darwin.dylib`).
fn download_dylib(dest_name: &str, dest: &Path) -> Result<(), String> {
	// ── Determine the target triple ──────────────────────────────
	let triple = target_triple();
	// Build the remote asset name from the destination filename.
	// e.g. libaphrodite.dylib → libaphrodite-aarch64-apple-darwin.dylib
	let ext = if cfg!(windows) {
		"dll"
	} else if cfg!(target_os = "macos") {
		"dylib"
	} else {
		"so"
	};
	let base = dest_name.strip_suffix(&format!(".{ext}")).unwrap_or(dest_name);
	let remote_name = format!("{base}-{triple}.{ext}");

	let version = env!("CARGO_PKG_VERSION");
	let release_dir = format!("https://github.com/PlayForm/Aphrodite/releases/download/Aphrodite/v{version}");
	let url = format!("{release_dir}/{remote_name}");

	println!("downloading {remote_name} from GitHub Releases...");
	println!("  url: {url}");

	// Use curl (available on macOS + Linux by default; Windows has it
	// in Git Bash / winget).  Fall back to PowerShell on Windows.
	let status = if cfg!(windows) {
		Command::new("powershell")
			.args([
				"-Command",
				&format!("Invoke-WebRequest -Uri '{url}' -OutFile '{}'", dest.display()),
			])
			.status()
	} else {
		Command::new("curl")
			.args(["-fsSL", "--retry", "3", "-o", dest.to_str().unwrap_or("dylib"), &url])
			.status()
	};

	match status {
		Ok(s) if s.success() => {
			println!("  downloaded -> {}", dest.display());
			// SHA256SUMS-verified (F3): a missing sums file (e.g. a release
			// cut before this was added) degrades to a loud warning rather
			// than a hard failure, matching download.sh's own tolerance for
			// older tags - see verify_download_checksum.
			if let Err(e) = verify_download_checksum(&release_dir, triple, &remote_name, dest) {
				let _ = fs::remove_file(dest);
				return Err(e);
			}
			#[cfg(unix)]
			secure_perms(dest, 0o755).map_err(|e| e.to_string())?;
			Ok(())
		},
		Ok(s) => Err(format!("download failed with exit code {}", s.code().unwrap_or(-1))),
		Err(e) => Err(format!("could not run download command: {e}")),
	}
}

/// Fetch `SHA256SUMS-<triple>.txt` from the same release and verify `dest`
/// against the entry for `asset_name`. Shells out to the platform's own
/// hashing tool (`shasum`/`sha256sum`/`Get-FileHash`) rather than adding a
/// crate dependency, matching this function's existing curl/PowerShell
/// shell-out pattern.
fn verify_download_checksum(release_dir: &str, triple: &str, asset_name: &str, dest: &Path) -> Result<(), String> {
	let sums_url = format!("{release_dir}/SHA256SUMS-{triple}.txt");
	let sums_text = if cfg!(windows) {
		Command::new("powershell")
			.args([
				"-Command",
				&format!("(Invoke-WebRequest -Uri '{sums_url}' -UseBasicParsing).Content"),
			])
			.output()
	} else {
		Command::new("curl").args(["-fsSL", &sums_url]).output()
	};
	let sums_text = match sums_text {
		Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
		_ => {
			println!("  WARNING: {sums_url} not found - skipping checksum verification for this release");
			return Ok(());
		},
	};
	let expected = sums_text.lines().find_map(|line| {
		let mut parts = line.split_whitespace();
		let hash = parts.next()?;
		let name = parts.next()?;
		(name == asset_name).then(|| hash.to_lowercase())
	});
	let Some(expected) = expected else {
		println!("  WARNING: {asset_name} has no entry in SHA256SUMS-{triple}.txt - skipping checksum check");
		return Ok(());
	};

	let hash_output = if cfg!(windows) {
		Command::new("powershell")
			.args([
				"-Command",
				&format!("(Get-FileHash '{}' -Algorithm SHA256).Hash", dest.display()),
			])
			.output()
	} else if Command::new("shasum").arg("--version").output().is_ok() {
		Command::new("shasum").args(["-a", "256", dest.to_str().unwrap_or("")]).output()
	} else {
		Command::new("sha256sum").arg(dest.to_str().unwrap_or("")).output()
	};
	let actual = match hash_output {
		Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
			.split_whitespace()
			.next()
			.unwrap_or("")
			.to_lowercase(),
		_ => return Err("no shasum/sha256sum/Get-FileHash available to verify checksum".to_string()),
	};

	if actual != expected {
		return Err(format!("checksum mismatch for {asset_name}: expected {expected}, got {actual}"));
	}
	println!("  checksum verified");
	Ok(())
}

/// Return the Rust target triple for the current platform.
fn target_triple() -> &'static str {
	if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
		"aarch64-apple-darwin"
	} else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
		"x86_64-apple-darwin"
	} else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
		"x86_64-unknown-linux-gnu"
	} else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
		"x86_64-pc-windows-msvc"
	} else {
		"unknown"
	}
}

/// Write plugin.yaml manifest.
fn write_plugin_yaml(ctx: &SetupCtx, args: &SetupArgs) -> Result<(), SetupError> {
	let path = ctx.aphrodite_dir.join("plugin.yaml");
	if path.exists() {
		return Ok(());
	}

	let yaml = format!(
		r#"name: aphrodite
version: {version}
description: "CCR compression plugin - 13 tools, context engine, TOML-driven templates."
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
  - aphrodite_directive
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
///
/// Embeds `plugins/aphrodite/__init__.py` (the monorepo's live Hermes
/// plugin) directly, rather than maintaining a separate hand-copied template
/// (report 07 F14/T8) - a hand-maintained second copy had drifted
/// significantly: stale dylib-reload/free_string handling (report 06 F1),
/// a `register_tool` call with the wrong argument count, no skills
/// registration, no version handshake, no port-env reads, no health poll,
/// and stderr piped to `DEVNULL` (silently re-introducing a startup-failure
/// bug the live plugin had already fixed). `include_str!` of the real file
/// means the two can never diverge again.
const HERMES_PLUGIN_SHIM: &str = include_str!("../templates/__init__.py");

fn write_init_py(ctx: &SetupCtx) -> Result<(), SetupError> {
	let path = ctx.aphrodite_dir.join("__init__.py");
	if path.exists() {
		return Ok(());
	}

	println!("writing __init__.py -> {}", path.display());
	fs::write(&path, HERMES_PLUGIN_SHIM)?;
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

#[cfg(test)]
mod tests {
	use super::*;

	// ── T8 (F14): the setup-embedded shim is now `include_str!`'d directly
	// from the live plugin, so it can't drift - these pin the specific bugs
	// the old hand-copied `templates/__init__.py` had regrown. ──
	#[test]
	fn test_hermes_plugin_shim_has_no_stderr_devnull() {
		assert!(
			!HERMES_PLUGIN_SHIM.contains("stderr=subprocess.DEVNULL"),
			"stderr must go to a log file, not DEVNULL (re-introduces the v1.2.1 silent-startup bug)"
		);
	}

	#[test]
	fn test_hermes_plugin_shim_reads_no_auto_launch() {
		assert!(
			HERMES_PLUGIN_SHIM.contains(r#"os.environ.get("APHRODITE_NO_AUTO_LAUNCH""#),
			"the guard must be read, not just set"
		);
	}

	#[test]
	fn test_hermes_plugin_shim_reads_port_env_vars() {
		assert!(HERMES_PLUGIN_SHIM.contains("APHRODITE_CACHE_PORT"));
		assert!(HERMES_PLUGIN_SHIM.contains("APHRODITE_TOKEN_PORT"));
	}

	#[test]
	fn test_hermes_plugin_shim_gates_context_engine_opt_in() {
		assert!(HERMES_PLUGIN_SHIM.contains("APHRODITE_CONTEXT_ENGINE"));
	}

	#[test]
	fn test_hermes_plugin_shim_registers_tools_with_toolset_arg() {
		// The old template called `ctx.register_tool(schema, handler)` (2
		// args) while the real Hermes API + live plugin use
		// `register_tool(name, toolset, schema, handler)` (4 args).
		assert!(HERMES_PLUGIN_SHIM.contains(r#"ctx.register_tool(name, "aphrodite", schema, "#));
	}

	// Network-touching (real GitHub release) - not run by default, only on
	// demand (`cargo test -p aphrodite --lib -- --ignored`) since the
	// hermetic suite must stay network-free.
	#[test]
	#[ignore = "hits the real GitHub release - run explicitly to verify"]
	fn test_verify_download_checksum_against_real_release() {
		let dir = std::env::temp_dir().join("aphrodite-checksum-test");
		fs::create_dir_all(&dir).unwrap();
		let release_dir = "https://github.com/PlayForm/Aphrodite/releases/download/Aphrodite/v1.3.2";
		let triple = "aarch64-apple-darwin";
		let asset = "aphrodite-aarch64-apple-darwin";
		let dest = dir.join(asset);
		let status = Command::new("curl")
			.args(["-fsSL", "-o", dest.to_str().unwrap(), &format!("{release_dir}/{asset}")])
			.status()
			.unwrap();
		assert!(status.success());

		// Correct hash passes.
		verify_download_checksum(release_dir, triple, asset, &dest).expect("real asset must verify clean");

		// A deliberately corrupted file must be rejected, not silently accepted.
		fs::write(&dest, b"corrupted content").unwrap();
		let result = verify_download_checksum(release_dir, triple, asset, &dest);
		assert!(result.is_err(), "corrupted asset must fail checksum verification");

		fs::remove_dir_all(&dir).unwrap();
	}
}
