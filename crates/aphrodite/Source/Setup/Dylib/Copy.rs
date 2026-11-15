//! The shared-library install pass.
//!
//! Searches the places a `cargo build` leaves the dylibs - next to the setup
//! binary, in `deps/`, and in the two system library directories - and falls
//! back to downloading from the GitHub release matching this binary's version,
//! so `cargo install aphrodite && aphrodite setup` works with no source
//! checkout. The core `libaphrodite` is best-effort: it exists for external
//! embedders, is not published as a release asset, and the Hermes plugin never
//! loads it. `libaphrodite_hermes` is hard-required - without it the plugin
//! cannot load at all.

#[cfg(not(target_os = "macos"))]
use std::fs;
use std::path::{Path, PathBuf};

use super::Download;
use crate::Setup::Run::{Context::Struct as SetupContext, Error::Enum as SetupError};
#[cfg(target_os = "macos")]
use crate::Setup::MacOS::Fn as InstallMacOSArtifact;
#[cfg(not(target_os = "macos"))]
use crate::Setup::Write::Permissions::Fn as SecurePerms;

/// Copy both dylibs from a build output directory, or download what is missing.
///
/// Release assets carry the target triple and the installed file must not:
/// `libaphrodite_hermes-aarch64-apple-darwin.dylib` is fetched and saved as
/// `libaphrodite_hermes.dylib`, which is the name the loader opens. A local hit
/// takes the platform's copy treatment; on macOS that is `MacOS::Fn`, which is
/// what keeps the copied dylib loadable instead of quarantine-killed.
///
/// It is not transactional: artifacts copied before a failure stay in place, so
/// a failed run can leave a partial `binaries/` directory and a re-run is the
/// repair. It also never removes a dylib it did not install - an optional core
/// library that cannot be found is skipped with a warning, because setup
/// completing without it is the correct outcome and aborting there would make
/// the documented `cargo install` path unusable.
pub(crate) fn Fn(Context:&SetupContext) -> Result<(), SetupError> {
	// The core `libaphrodite` cdylib is BEST-EFFORT: it exists for external
	// embedders, is not published on GitHub Releases (Build.yml ships only
	// `aphrodite-<target>` + `libaphrodite_hermes-<target>`), and the Hermes
	// plugin loads only `libaphrodite_hermes`.  A missing core dylib must
	// never abort setup - warn and continue.  `libaphrodite_hermes` is
	// hard-required: without it the plugin cannot load.
	let DylibNames:&[(&str, bool)] = if cfg!(target_os = "macos") {
		&[("libaphrodite.dylib", false), ("libaphrodite_hermes.dylib", true)]
	} else if cfg!(target_os = "linux") {
		&[("libaphrodite.so", false), ("libaphrodite_hermes.so", true)]
	} else {
		&[("aphrodite.dll", false), ("aphrodite_hermes.dll", true)]
	};

	let ExeDirectory = Context.OwnPath.parent().unwrap_or(Path::new("."));
	let SearchPaths:Vec<PathBuf> = vec![
		ExeDirectory.to_path_buf(),
		ExeDirectory.join("deps"),
		PathBuf::from("/usr/local/lib"),
		PathBuf::from("/opt/homebrew/lib"),
	];

	let mut Copied = 0u32;
	for &(name, Required) in DylibNames {
		let Dest = Context.BinariesDirectory.join(name);

		let mut Found = false;
		for SearchDirectory in &SearchPaths {
			let Src = SearchDirectory.join(name);
			if Src.exists() {
				println!("copying dylib {} -> {}", name, Dest.display());
				// Fix install name: `cargo build` embeds the target/deps/
				// path as the dylib's ID. Loading a copied dylib whose ID
				// points to a non-existent (or stale) build-directory path
				// causes the macOS dynamic linker to SIGKILL the process.
				// See `MacOS::Fn`'s doc comment (03-F6/F7/F9) for why this
				// isn't just `ditto`/`install_name_tool` + `let _ =`.
				#[cfg(target_os = "macos")]
				InstallMacOSArtifact(&Src, &Dest, Some(name), 0o755)?;
				#[cfg(not(target_os = "macos"))]
				{
					fs::copy(&Src, &Dest)?;
					SecurePerms(&Dest, 0o755)?;
				}
				Found = true;
				Copied += 1;
				break;
			}
		}
		if !Found {
			let TargetRelease = ExeDirectory
				.parent()
				.unwrap_or(Path::new("."))
				.parent()
				.unwrap_or(Path::new("."))
				.join("target")
				.join("release")
				.join(name);
			if TargetRelease.exists() {
				println!("copying dylib {} -> {}", name, Dest.display());
				// 03-F7: this fallback (dev builds where the dylib isn't
				// co-located with the setup binary) used to skip the
				// Gatekeeper treatment above entirely - same helper here too.
				#[cfg(target_os = "macos")]
				InstallMacOSArtifact(&TargetRelease, &Dest, Some(name), 0o755)?;
				#[cfg(not(target_os = "macos"))]
				{
					fs::copy(&TargetRelease, &Dest)?;
					SecurePerms(&Dest, 0o755)?;
				}
				Found = true;
				Copied += 1;
			}
		}
		if !Found {
			// Local search exhausted - try downloading from the GitHub
			// release matching this binary's version.  `cargo install`
			// only delivers [[bin]] targets, never cdylib artifacts, so
			// a `cargo install aphrodite && aphrodite setup` user will
			// always land here.  This download path bridges that gap
			// without requiring a full source checkout.
			if let Err(e) = Download::Fn(name, &Dest) {
				if Required {
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
			Copied += 1;
		}
	}

	println!("copied {Copied} dylib(s)");
	Ok(())
}
