use std::{fs, path::Path, process::Command};

use super::*;

/// Compute BLAKE3 hash of the binary for integrity display.
pub(crate) fn self_hash(path:&Path) -> String {
	match fs::read(path) {
		Ok(bytes) => {
			let hash = blake3::hash(&bytes);
			hash.to_hex().to_string()
		},
		Err(_) => "unknown".into(),
	}
}

/// Verify hermes CLI is available.
pub(crate) fn verify_hermes() -> Result<(), SetupError> {
	match Command::new("hermes").arg("--version").output() {
		Ok(out) if out.status.success() => {
			let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
			println!("  hermes found: {version}");
			Ok(())
		},
		Ok(out) => {
			let stderr = String::from_utf8_lossy(&out.stderr);
			Err(SetupError::HermesNotFound(format!("hermes --version failed: {stderr}")))
		},
		Err(_) => {
			Err(SetupError::HermesNotFound(
				"hermes not found in PATH - install hermes agent first".into(),
			))
		},
	}
}
