//! The binary's own integrity hash.
//!
//! Setup prints the BLAKE3 of the artifact it is about to install so a user can
//! tell two installs of the same version apart - a locally built binary and a
//! released one share a version string and nothing else. It is display-only
//! information; nothing in the install path compares it against anything.

use std::{fs, path::Path};

/// The BLAKE3 hash of `path`, hex-encoded, or `"unknown"`.
///
/// `"unknown"` is returned when the file cannot be read - the running binary is
/// always readable, so that arm exists for a caller passing some other path.
///
/// It is not a verification step: setup never checks this hash against a
/// published value, so a wrong hash here misinforms the user but cannot install
/// the wrong bytes. Failing the install on an unreadable self would be worse than
/// printing `"unknown"`, which is why the error is swallowed rather than
/// propagated.
pub(crate) fn Fn(path:&Path) -> String {
	match fs::read(path) {
		Ok(bytes) => {
			let hash = blake3::hash(&bytes);
			hash.to_hex().to_string()
		},
		Err(_) => "unknown".into(),
	}
}
