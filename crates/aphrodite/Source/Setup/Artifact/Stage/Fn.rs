//! Downloading a release asset into a staging path.

use std::{path::Path, process::Command};

use super::super::Staged;

/// Fetch `Url` into a staging path next to `Dest`.
///
/// The bytes land in a hidden sibling of the destination (`.<name>.partial`), never at
/// the destination itself. That is what makes an interrupted download harmless: the
/// final path either holds the previous artifact or nothing, and a truncated transfer
/// can never be mistaken for an installed dylib.
///
/// `curl --retry 3` covers a flaky transfer; anything beyond that is reported. A stale
/// partial from an earlier run is removed first, so a failed download cannot leave bytes
/// that a later run's verification would be asked about.
///
/// Windows has no `curl` guarantee, so it uses PowerShell; every other target uses
/// `curl`, which both macOS and Linux ship. `#[cfg]` selects the command, so there is no
/// platform branch in the flow.
///
/// The staging path is passed as an `OsStr` argument, never through `to_str`. The old
/// call was `Partial.to_str().unwrap_or("dylib")`, which on a path that is not valid
/// UTF-8 quietly downloaded the asset to a file named `dylib` in the working directory
/// and then reported the *staging* path as the result - a successful-looking install of
/// the wrong file in the wrong place.
pub(crate) fn Fn(Url:&str, Name:&str, Dest:&Path) -> anyhow::Result<Staged::Struct::Struct> {
	let Partial = Dest.with_file_name(format!(".{Name}.partial"));

	let _ = std::fs::remove_file(&Partial);

	#[cfg(windows)]
	let Status = Command::new("powershell")
		.args([
			"-Command",
			&format!("Invoke-WebRequest -Uri '{Url}' -OutFile '{}'", Partial.display()),
		])
		.status();

	#[cfg(not(windows))]
	let Status = Command::new("curl")
		.args(["-fsSL", "--retry", "3"])
		.arg("-o")
		.arg(&Partial)
		.arg(Url)
		.status();

	match Status {
		Ok(s) if s.success() => Ok(Staged::Struct::Struct { Path:Partial, Dest:Dest.to_path_buf() }),
		Ok(s) => {
			let _ = std::fs::remove_file(&Partial);

			anyhow::bail!("download failed with exit code {}", s.code().unwrap_or(-1))
		},
		Err(e) => {
			let _ = std::fs::remove_file(&Partial);

			Err(anyhow::Error::new(e).context(format!("run the download command for {Name}")))
		},
	}
}
