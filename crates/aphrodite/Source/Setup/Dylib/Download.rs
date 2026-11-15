//! Release-asset download for a dylib that is not on disk.
//!
//! The asset name is built from the destination filename plus the target
//! triple, because a release publishes one file per platform while the loader
//! opens one fixed name. Fetching uses the platform's own tool - `curl` on
//! macOS and Linux, `Invoke-WebRequest` on Windows - rather than a HTTP crate,
//! so the install path adds no dependency to the binary.

use std::{fs, path::Path, process::Command};

#[cfg(unix)]
use crate::Setup::Write::Permissions::Fn as SecurePerms;
use super::{Checksum, TargetTriple};

/// Download one dylib from the GitHub release matching this binary's version.
///
/// `DestName` is the bare filename the loader opens (`libaphrodite.dylib`) and
/// `Dest` is where it lands; the remote asset is the tripled name
/// (`libaphrodite-aarch64-apple-darwin.dylib`), derived from `DestName` and the
/// platform's triple. The bytes are checked against the release's
/// `SHA256SUMS-<triple>.txt` before the file is kept.
///
/// It is not a retry loop: `curl --retry 3` covers a flaky transfer and anything
/// beyond that is reported as an error. A checksum failure deletes the partial
/// file before returning, so a later step cannot install bytes that were never
/// verified - leaving a corrupt dylib behind would make the plugin fail to load
/// on every subsequent start with nothing pointing at the download.
///
/// Returns `anyhow::Result` rather than the setup error type because the caller
/// decides whether the failure is fatal: the Hermes dylib is required, the core
/// one is not.
pub(super) fn Fn(DestName:&str, Dest:&Path) -> anyhow::Result<()> {
	// ── Determine the target triple ──────────────────────────────
	let Triple = TargetTriple::Fn();
	// Build the remote asset name from the destination filename.
	// e.g. libaphrodite.dylib → libaphrodite-aarch64-apple-darwin.dylib
	let Ext = if cfg!(windows) {
		"dll"
	} else if cfg!(target_os = "macos") {
		"dylib"
	} else {
		"so"
	};
	let Base = DestName.strip_suffix(&format!(".{Ext}")).unwrap_or(DestName);
	let RemoteName = format!("{Base}-{Triple}.{Ext}");

	let version = env!("CARGO_PKG_VERSION");
	let ReleaseDirectory = format!("https://github.com/PlayForm/Aphrodite/releases/download/Aphrodite/v{version}");
	let url = format!("{ReleaseDirectory}/{RemoteName}");

	println!("downloading {RemoteName} from GitHub Releases...");
	println!("  url: {url}");

	// Use curl (available on macOS + Linux by default; Windows has it
	// in Git Bash / winget).  Fall back to PowerShell on Windows.
	let Status = if cfg!(windows) {
		Command::new("powershell")
			.args([
				"-Command",
				&format!("Invoke-WebRequest -Uri '{url}' -OutFile '{}'", Dest.display()),
			])
			.status()
	} else {
		Command::new("curl")
			.args(["-fsSL", "--retry", "3", "-o", Dest.to_str().unwrap_or("dylib"), &url])
			.status()
	};

	match Status {
		Ok(s) if s.success() => {
			println!("  downloaded -> {}", Dest.display());
			// SHA256SUMS-verified (F3 + PR 118488 catalog review): the sums file is
			// MANDATORY - a missing file, a missing asset entry, or a mismatch is a
			// hard refusal (Err -> setup exits non-zero), never a skipped check,
			// matching download.sh/download.ps1's in-tree sums doctrine.
			if let Err(e) = Checksum::Fn(&ReleaseDirectory, Triple, &RemoteName, Dest) {
				let _ = fs::remove_file(Dest);
				return Err(e.context(format!("verify the checksum of {RemoteName}")));
			}
			#[cfg(unix)]
			SecurePerms(Dest, 0o755)?;
			Ok(())
		},
		Ok(s) => anyhow::bail!("download failed with exit code {}", s.code().unwrap_or(-1)),
		Err(e) => Err(anyhow::Error::new(e).context(format!("run the download command for {RemoteName}"))),
	}
}
