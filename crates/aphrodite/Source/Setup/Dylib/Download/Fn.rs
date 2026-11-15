//! Release-asset download for a dylib that is not on disk.
//!
//! The asset name is built from the destination filename plus the target triple,
//! because a release publishes one file per platform while the loader opens one
//! fixed name. Fetching uses the platform's own tool rather than a HTTP crate, so
//! the install path adds no dependency to the binary.
//!
//! The download is the four-step pipeline from `Setup::Artifact` - stage, verify,
//! prepare, publish - and this function is the only place it is wired together.

use std::path::Path;

use crate::Setup::Artifact;
use super::super::TargetTriple;

/// Download one dylib from the GitHub release matching this binary's version.
///
/// `DestName` is the bare filename the loader opens (`libaphrodite.dylib`) and
/// `Dest` is where it lands; the remote asset is the tripled name
/// (`libaphrodite-aarch64-apple-darwin.dylib`), derived from `DestName` and the
/// platform's triple.
///
/// It is not a retry loop: `curl --retry 3` covers a flaky transfer and anything
/// beyond that is reported as an error. A checksum failure deletes the staged file
/// before returning, and the bytes never reach `Dest` at all - they are staged as a
/// hidden sibling and renamed into place only after verification and chmod. Leaving
/// a corrupt dylib at the install path would make the plugin fail to load on every
/// subsequent start with nothing pointing at the download.
///
/// Returns `anyhow::Result` rather than a setup-specific error type because the
/// caller decides whether the failure is fatal: the Hermes dylib is required, the
/// core one is not.
pub(crate) fn Fn(DestName:&str, Dest:&Path) -> anyhow::Result<()> {
	let Triple = TargetTriple::Const::Const;

	#[cfg(windows)]
	let Extension = "dll";
	#[cfg(all(not(windows), target_os = "macos"))]
	let Extension = "dylib";
	#[cfg(all(not(windows), not(target_os = "macos")))]
	let Extension = "so";

	let Base = DestName.strip_suffix(&format!(".{Extension}")).unwrap_or(DestName);
	let RemoteName = format!("{Base}-{Triple}.{Extension}");

	let ReleaseDirectory = format!(
		"https://github.com/PlayForm/Aphrodite/releases/download/Aphrodite/v{}",
		env!("CARGO_PKG_VERSION")
	);
	let Url = format!("{ReleaseDirectory}/{RemoteName}");

	println!("downloading {RemoteName} from GitHub Releases...");
	println!("  url: {Url}");

	let Staged = Artifact::Stage::Fn::Fn(&Url, DestName, Dest)?;
	let Verified = Artifact::Verify::Fn::Fn(Staged, &ReleaseDirectory, Triple, &RemoteName)?;
	let Prepared = Artifact::Prepare::Fn::Fn(Verified, 0o755)?;
	let Published = Artifact::Publish::Fn::Fn(Prepared)?;

	println!("  downloaded -> {}", Published.Path.display());

	Ok(())
}
