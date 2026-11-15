//! Copying the running binary into `binaries/`.

#[cfg(not(target_os = "macos"))]
use std::fs;

#[cfg(not(target_os = "macos"))]
use anyhow::Context;

use crate::Setup::{Run::Context::Struct as SetupContext, Write::BinaryName};
#[cfg(not(target_os = "macos"))]
use crate::Setup::Write::Permissions::Fn as SecurePerms;
#[cfg(target_os = "macos")]
use crate::Setup::MacOS::Fn as InstallMacOSArtifact;

/// Install this executable as the runtime's proxy binary.
///
/// Always overwritten: the binary is the install payload, and only `aphrodite.toml`
/// is preserved unless `--force`. A stale binary beside a fresh dylib is a version
/// handshake failure at plugin start with nothing pointing at the install.
///
/// macOS takes `MacOS::Fn` because `fs::copy` preserves extended attributes (code
/// signature, quarantine) from the build directory, and Gatekeeper kills the copied
/// binary at the install path. See that function's doc comment (03-F6/F7/F9) for why
/// this is not just `ditto` plus `let _ =`.
pub(crate) fn Fn(Context:&SetupContext) -> anyhow::Result<()> {
	let TargetBinary = Context.BinariesDirectory.join(BinaryName::Const);

	println!("copying binary -> {}", TargetBinary.display());

	#[cfg(target_os = "macos")]
	InstallMacOSArtifact(&Context.OwnPath, &TargetBinary, None, 0o700)?;

	#[cfg(not(target_os = "macos"))]
	{
		fs::copy(&Context.OwnPath, &TargetBinary)
			.with_context(|| format!("copy this executable to {}", TargetBinary.display()))?;
		SecurePerms(&TargetBinary, 0o700)?;
	}

	Ok(())
}
