//! Gatekeeper-safe artifact install for macOS.
//!
//! Every file setup puts on a macOS install path - the binary and both dylibs - goes
//! through this one function, because the treatment is not optional: `fs::copy`
//! preserves the quarantine attribute and the stale install name, and Gatekeeper kills
//! the process that loads the result. It is not a general copy helper and it is not
//! called on any other platform; the non-macOS paths copy and chmod directly.

use std::{fs, path::Path, process::Command};

use anyhow::Context;

use crate::{Logging::Stderr::Fn::Fn as StderrLine, Setup::Write::Permissions::Fn as SecurePerms};

/// Copy `Src` to `Dest` with the full macOS treatment, then harden permissions.
///
/// `DylibIDName` is `Some(name)` for a dylib copy - the install name is rewritten to
/// `@rpath/<name>` and the artifact is ad-hoc re-signed, because `install_name_tool`
/// invalidates the embedded signature on arm64 - and `None` for a plain binary copy,
/// which has no dylib ID to rewrite and no signature-invalidating relink. The copy
/// itself prefers `ditto` and falls back to `fs::copy` plus `xattr -c`.
///
/// It is not best-effort. A non-zero exit from `ditto` counts as failure and takes the
/// fallback, and every tool that is missing or fails warns with the consequence named,
/// because a swallowed `let _ =` here is what made the SIGKILL bug look intermittent on
/// source builds. Getting this wrong leaves an artifact at the install path that macOS
/// refuses to load, and the only symptom is a plugin that never starts.
///
/// Each tool's outcome is folded into the one fact the code branches on - did it
/// succeed - with `is_ok_and`, so a spawn failure and a non-zero exit are the same
/// answer and neither needs its own arm.
///
/// Every path is passed as an `OsStr` argument. The old calls went through
/// `Dest.to_str().unwrap_or("")`, which on a path that is not valid UTF-8 handed the
/// tools an empty string: `ditto` copied nothing, `xattr` cleaned nothing, and
/// `install_name_tool`/`codesign` silently "succeeded" against no file.
#[cfg(target_os = "macos")]
pub(crate) fn Fn(Src:&Path, Dest:&Path, DylibIDName:Option<&str>, Mode:u32) -> anyhow::Result<()> {
	let _ = fs::remove_file(Dest);

	let DittoOk = Command::new("ditto").arg(Src).arg(Dest).status().is_ok_and(|s| s.success());

	if !DittoOk {
		fs::copy(Src, Dest).with_context(|| format!("copy {} to {}", Src.display(), Dest.display()))?;

		let XattrOk = Command::new("xattr")
			.arg("-c")
			.arg(Dest)
			.output()
			.is_ok_and(|o| o.status.success());

		if !XattrOk {
			StderrLine(&format!(
				"warning: xattr -c failed or unavailable for {} - Gatekeeper may still kill this artifact",
				Dest.display()
			));
		}
	}

	// Only a dylib has an install name to rewrite and a signature to restore.
	if let Some(name) = DylibIDName {
		let Rpath = format!("@rpath/{name}");

		let Rewritten = Command::new("install_name_tool")
			.args(["-id", &Rpath])
			.arg(Dest)
			.output()
			.is_ok_and(|o| o.status.success());

		if !Rewritten {
			StderrLine(&format!(
				"warning: install_name_tool failed or unavailable for {} - Gatekeeper may kill Hermes when loading \
				 this dylib; install Xcode Command Line Tools and re-run setup",
				Dest.display()
			));
		}

		// Ad-hoc re-sign: `install_name_tool` invalidates the dylib's embedded signature
		// on arm64.
		let Signed = Command::new("codesign")
			.args(["-f", "-s", "-"])
			.arg(Dest)
			.output()
			.is_ok_and(|o| o.status.success());

		if !Signed {
			StderrLine(&format!(
				"warning: codesign failed or unavailable for {} - Gatekeeper may still kill this dylib",
				Dest.display()
			));
		}
	}

	SecurePerms(Dest, Mode)?;

	Ok(())
}
