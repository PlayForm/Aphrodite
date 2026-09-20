use std::{
	fs,
	path::{Path, PathBuf},
	process::Command,
};

use super::*;

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
pub(crate) fn copy_dylibs(ctx:&SetupCtx) -> Result<(), SetupError> {
	// The core `libaphrodite` cdylib is BEST-EFFORT: it exists for external
	// embedders, is not published on GitHub Releases (Build.yml ships only
	// `aphrodite-<target>` + `libaphrodite_hermes-<target>`), and the Hermes
	// plugin loads only `libaphrodite_hermes`.  A missing core dylib must
	// never abort setup - warn and continue.  `libaphrodite_hermes` is
	// hard-required: without it the plugin cannot load.
	let dylib_names:&[(&str, bool)] = if cfg!(target_os = "macos") {
		&[("libaphrodite.dylib", false), ("libaphrodite_hermes.dylib", true)]
	} else if cfg!(target_os = "linux") {
		&[("libaphrodite.so", false), ("libaphrodite_hermes.so", true)]
	} else {
		&[("aphrodite.dll", false), ("aphrodite_hermes.dll", true)]
	};

	let exe_dir = ctx.own_path.parent().unwrap_or(Path::new("."));
	let search_paths:Vec<PathBuf> = vec![
		exe_dir.to_path_buf(),
		exe_dir.join("deps"),
		PathBuf::from("/usr/local/lib"),
		PathBuf::from("/opt/homebrew/lib"),
	];

	let mut copied = 0u32;
	for &(name, required) in dylib_names {
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
				if required {
					return Err(SetupError::DylibNotFound(format!(
						"dylib '{name}' not found locally and download failed: {e}.  \
						 Build from source (cargo build --release -p aphrodite -p aphrodite-hermes) \
						 or download manually from \
						 https://github.com/PlayForm/Aphrodite/releases/tag/Aphrodite/v{version}",
						version = env!("CARGO_PKG_VERSION"),
					)));
				}
				// Optional core dylib: degrade gracefully (F9) - the
				// Hermes plugin runtime never loads it, so setup can
				// complete without it.
				eprintln!(
					"WARNING: optional dylib '{name}' could not be located or downloaded ({e}); continuing without it \
					 - the Hermes plugin loads only the *_hermes dylib"
				);
				continue;
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
fn download_dylib(dest_name:&str, dest:&Path) -> Result<(), String> {
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
pub(crate) fn verify_download_checksum(
	release_dir:&str,
	triple:&str,
	asset_name:&str,
	dest:&Path,
) -> Result<(), String> {
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
		Ok(o) if o.status.success() => {
			String::from_utf8_lossy(&o.stdout)
				.split_whitespace()
				.next()
				.unwrap_or("")
				.to_lowercase()
		},
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
