//! File permission hardening for everything setup installs.
//!
//! The configuration file carries an API key and the proxy ports, so the install
//! path sets an explicit mode on every artifact it writes instead of inheriting the
//! process umask. Every write step ends here; none of them relies on the default.

use std::{fs, path::Path};

use anyhow::Context;

/// Set `path` to `Mode`, Unix only.
///
/// Setup's modes are deliberate: `0o600` for `aphrodite.toml`, `0o644` for the
/// loader, manifest and version pin, `0o700` for the binary and `0o755` for the
/// dylibs.
///
/// It is not an ownership change and it is not a recursive operation - only the
/// one path named is touched, so a directory the user already owns keeps its
/// ownership and a nested file needs its own call. On non-Unix targets the call is
/// a no-op that returns `Ok`: Windows has no mode bits, and failing there would
/// make setup unusable rather than safer.
pub(crate) fn Fn(path:&Path, Mode:u32) -> anyhow::Result<()> {
	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		let mut Perms = fs::metadata(path)
			.with_context(|| format!("read the permissions of {}", path.display()))?
			.permissions();
		Perms.set_mode(Mode);
		fs::set_permissions(path, Perms).with_context(|| format!("set mode {Mode:o} on {}", path.display()))?;
	}
	#[cfg(not(unix))]
	let _ = (path, Mode);
	Ok(())
}
