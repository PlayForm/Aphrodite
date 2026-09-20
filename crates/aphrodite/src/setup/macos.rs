use std::{fs, path::Path, process::Command};

use super::*;

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
pub(crate) fn install_macos_artifact(
	src:&Path,
	dest:&Path,
	dylib_id_name:Option<&str>,
	mode:u32,
) -> Result<(), SetupError> {
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
