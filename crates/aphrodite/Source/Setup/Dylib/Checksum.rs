//! Checksum verification for a downloaded dylib.
//!
//! A download is only installed after its bytes match the entry in the
//! release's `SHA256SUMS-<triple>.txt`. Hashing shells out to the platform's own
//! tool (`shasum`, `sha256sum`, `Get-FileHash`) instead of adding a crate
//! dependency, matching the way the download itself shells out to `curl`.

use std::{path::Path, process::Command};

/// Verify `Dest` against the `SHA256SUMS-<triple>.txt` entry for `AssetName`.
///
/// The sums file is fetched from the same release directory the asset came from,
/// so a mirror cannot serve bytes the release does not vouch for. Both the file
/// and the asset's entry in it are mandatory.
///
/// It is not a best-effort check and it has no "skip verification" path: a
/// missing sums file, a missing entry, or a hash mismatch all return `Err`, which
/// makes setup exit non-zero. The rule is deliberate - an asset that cannot be
/// validated must never be installed, and a silently skipped check is
/// indistinguishable from a passing one at every later step.
pub(crate) fn Fn(ReleaseDirectory:&str, Triple:&str, AssetName:&str, Dest:&Path) -> anyhow::Result<()> {
	let SumsURL = format!("{ReleaseDirectory}/SHA256SUMS-{Triple}.txt");
	let SumsText = if cfg!(windows) {
		Command::new("powershell")
			.args([
				"-Command",
				&format!("(Invoke-WebRequest -Uri '{SumsURL}' -UseBasicParsing).Content"),
			])
			.output()
	} else {
		Command::new("curl").args(["-fsSL", &SumsURL]).output()
	};
	let SumsText = match SumsText {
		Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
		_ => {
			// PR 118488 catalog review: the sums file is mandatory - a
			// missing file is a HARD REFUSAL, never a skipped check (a
			// release without its sums cannot be validated, so the
			// download must not proceed).
			anyhow::bail!(
				"SHA256SUMS-{Triple}.txt not found at {SumsURL} - refusing to install without the mandatory checksum \
				 list"
			);
		},
	};
	let Expected = SumsText.lines().find_map(|Line| {
		let mut Parts = Line.split_whitespace();
		let hash = Parts.next()?;
		let name = Parts.next()?;
		(name == AssetName).then(|| hash.to_lowercase())
	});
	let Some(Expected) = Expected else {
		// PR 118488 catalog review: an asset without a sums entry is a HARD
		// REFUSAL, never a skipped check - an unverifiable asset must not
		// be installed.
		anyhow::bail!(
			"{AssetName} has no entry in SHA256SUMS-{Triple}.txt - refusing to install an unverifiable asset"
		);
	};

	let HashOutput = if cfg!(windows) {
		Command::new("powershell")
			.args([
				"-Command",
				&format!("(Get-FileHash '{}' -Algorithm SHA256).Hash", Dest.display()),
			])
			.output()
	} else if Command::new("shasum").arg("--version").output().is_ok() {
		Command::new("shasum").args(["-a", "256", Dest.to_str().unwrap_or("")]).output()
	} else {
		Command::new("sha256sum").arg(Dest.to_str().unwrap_or("")).output()
	};
	let Actual = match HashOutput {
		Ok(o) if o.status.success() => {
			String::from_utf8_lossy(&o.stdout)
				.split_whitespace()
				.next()
				.unwrap_or("")
				.to_lowercase()
		},
		_ => anyhow::bail!("no shasum/sha256sum/Get-FileHash available to verify checksum"),
	};

	match Actual != Expected {
		true => {
			anyhow::bail!("checksum mismatch for {AssetName}: expected {Expected}, got {Actual}");
		},
		false => (),
	}
	println!("  checksum verified");
	Ok(())
}
