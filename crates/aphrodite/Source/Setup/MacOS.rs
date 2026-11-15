//! Gatekeeper-safe artifact install for macOS.
//!
//! Every file setup puts on a macOS install path - the binary and both dylibs -
//! goes through this one function, because the treatment is not optional:
//! `fs::copy` preserves the quarantine attribute and the stale install name, and
//! Gatekeeper kills the process that loads the result. It is not a general copy
//! helper and it is not called on any other platform; the non-macOS paths copy
//! and chmod directly.

use std::{fs, path::Path, process::Command};

use crate::Setup::{Run::Error::Enum as SetupError, Write::Permissions::Fn as SecurePerms};

/// Copy `Src` to `Dest` with the full macOS treatment, then harden permissions.
///
/// `DylibIDName` is `Some(name)` for a dylib copy - the install name is
/// rewritten to `@rpath/<name>` and the artifact is ad-hoc re-signed, because
/// `install_name_tool` invalidates the embedded signature on arm64 - and `None`
/// for a plain binary copy, which has no dylib ID to rewrite and no
/// signature-invalidating relink. The copy itself prefers `ditto` and falls back
/// to `fs::copy` plus `xattr -c`.
///
/// It is not best-effort. A non-zero exit from `ditto` counts as failure and
/// takes the fallback (`.status().is_err()` only catches a spawn failure), and
/// every tool that is missing or fails warns with the consequence named, because
/// a swallowed `let _ =` here is what made the SIGKILL bug look intermittent on
/// source builds. Getting this wrong leaves an artifact at the install path that
/// macOS refuses to load, and the only symptom is a plugin that never starts.
#[cfg(target_os = "macos")]
pub(crate) fn Fn(Src:&Path, Dest:&Path, DylibIDName:Option<&str>, Mode:u32) -> Result<(), SetupError> {
	let _ = std::fs::remove_file(Dest);
	let DittoOk = Command::new("ditto")
		.args([Src.to_str().unwrap_or(""), Dest.to_str().unwrap_or("")])
		.status()
		.map(|s| s.success())
		.unwrap_or(false);
	if !DittoOk {
		fs::copy(Src, Dest)?;
		match Command::new("xattr").args(["-c", Dest.to_str().unwrap_or("")]).output() {
			Ok(Out) if Out.status.success() => {},
			_ => {
				eprintln!(
					"warning: xattr -c failed or unavailable for {} - Gatekeeper may still kill this artifact",
					Dest.display()
				)
			},
		}
	}
	if let Some(name) = DylibIDName {
		let Rpath = format!("@rpath/{name}");
		match Command::new("install_name_tool")
			.args(["-id", &Rpath, Dest.to_str().unwrap_or("")])
			.output()
		{
			Ok(Out) if Out.status.success() => {},
			_ => {
				eprintln!(
					"warning: install_name_tool failed or unavailable for {} - Gatekeeper may kill Hermes when \
					 loading this dylib; install Xcode Command Line Tools and re-run setup",
					Dest.display()
				)
			},
		}
		// Ad-hoc re-sign: install_name_tool invalidates the dylib's embedded
		// signature on arm64.
		match Command::new("codesign")
			.args(["-f", "-s", "-", Dest.to_str().unwrap_or("")])
			.output()
		{
			Ok(Out) if Out.status.success() => {},
			_ => {
				eprintln!(
					"warning: codesign failed or unavailable for {} - Gatekeeper may still kill this dylib",
					Dest.display()
				)
			},
		}
	}
	SecurePerms(Dest, Mode)?;
	Ok(())
}
