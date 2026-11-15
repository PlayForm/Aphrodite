//! Installing one dylib that was found on disk.

use std::path::Path;
#[cfg(not(target_os = "macos"))]
use std::fs;

#[cfg(not(target_os = "macos"))]
use anyhow::Context;

#[cfg(target_os = "macos")]
use crate::Setup::MacOS::Fn as InstallMacOSArtifact;
#[cfg(not(target_os = "macos"))]
use crate::Setup::Write::Permissions::Fn as SecurePerms;

/// Copy one dylib into `Dest`, taking the platform's copy treatment.
///
/// On macOS that is `MacOS::Fn`, which rewrites the install name and re-signs the
/// artifact - `cargo build` embeds the `target/deps/` path as the dylib's ID, and
/// loading a copied dylib whose ID points at a non-existent build directory makes
/// the dynamic linker SIGKILL the process. Every other target copies and chmods.
///
/// Both branches are `#[cfg]`-selected, so only one of them is compiled: this is
/// the platform's copy, not a branch on the platform.
pub(crate) fn Fn(Src:&Path, Dest:&Path, Name:&str) -> anyhow::Result<()> {
	#[cfg(target_os = "macos")]
	InstallMacOSArtifact(Src, Dest, Some(Name), 0o755)?;

	#[cfg(not(target_os = "macos"))]
	{
		fs::copy(Src, Dest).with_context(|| format!("copy {Name} from {} to {}", Src.display(), Dest.display()))?;
		SecurePerms(Dest, 0o755)?;
	}

	Ok(())
}
