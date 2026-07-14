#![allow(unused_assignments)]

//! # aphrodite build.rs
//!
//! Embeds version, git commit hash, build timestamp, target triple, rustc
//! version, and build profile as compile-time environment variables
//! (`APHRODITE_*`) for runtime display in startup logs, health checks, and
//! version endpoints.
//!
//! CI (`Build.yml`) stages release artifacts per full target triple, e.g.
//!   aphrodite-x86_64-unknown-linux-gnu
//!   aphrodite-aarch64-apple-darwin
//!   aphrodite-x86_64-apple-darwin
//!   aphrodite-x86_64-pc-windows-msvc.exe

use std::{
	env,
	process::Command,
	time::{SystemTime, UNIX_EPOCH},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("cargo:rerun-if-changed=Cargo.toml");
	println!("cargo:rerun-if-changed=build.rs");

	// Version - Cargo already provides CARGO_PKG_VERSION to build scripts and
	// to the compiled crate (via `env!`), so just forward it rather than
	// re-parsing Cargo.toml.
	println!(
		"cargo:rustc-env=APHRODITE_VERSION={}",
		env::var("CARGO_PKG_VERSION").unwrap_or_default()
	);

	// Git commit hash (short form)
	if let Ok(output) = Command::new("git").args(["rev-parse", "--short", "HEAD"]).output() {
		if output.status.success() {
			let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
			if !hash.is_empty() {
				println!("cargo:rustc-env=APHRODITE_GIT_HASH={}", hash);
			}
		}
	}

	// Build timestamp (UTC ISO 8601), honoring SOURCE_DATE_EPOCH for
	// reproducible builds. Uses SystemTime instead of shelling out to `date`,
	// which isn't available on Windows runners.
	let epoch_secs = env::var("SOURCE_DATE_EPOCH")
		.ok()
		.and_then(|s| s.parse::<u64>().ok())
		.or_else(|| SystemTime::now().duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs()));
	if let Some(secs) = epoch_secs {
		println!("cargo:rustc-env=APHRODITE_BUILD_DATE={}", format_iso8601(secs));
	}

	// Target triple
	println!("cargo:rustc-env=APHRODITE_TARGET={}", env::var("TARGET").unwrap_or_default());

	// Rustc version
	if let Ok(output) = Command::new("rustc").arg("--version").output() {
		if output.status.success() {
			let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
			if !version.is_empty() {
				println!("cargo:rustc-env=APHRODITE_RUSTC_VERSION={}", version);
			}
		}
	}

	// Build profile (debug / release)
	println!("cargo:rustc-env=APHRODITE_PROFILE={}", env::var("PROFILE").unwrap_or_default());

	Ok(())
}

/// Formats a Unix timestamp (seconds) as UTC ISO 8601, e.g.
/// `2026-07-13T00:00:00Z`. Implemented by hand (civil-from-days algorithm) to
/// avoid adding a chrono/time build-dependency just for this.
fn format_iso8601(secs: u64) -> String {
	let days = (secs / 86400) as i64;
	let rem = secs % 86400;
	let (hour, minute, second) = (rem / 3600, (rem % 3600) / 60, rem % 60);

	// Howard Hinnant's civil_from_days algorithm.
	let z = days + 719468;
	let era = if z >= 0 { z } else { z - 146096 } / 146097;
	let doe = (z - era * 146097) as u64;
	let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
	let y = yoe as i64 + era * 400;
	let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
	let mp = (5 * doy + 2) / 153;
	let d = doy - (153 * mp + 2) / 5 + 1;
	let m = if mp < 10 { mp + 3 } else { mp - 9 };
	let y = if m <= 2 { y + 1 } else { y };

	format!("{y:04}-{m:02}-{d:02}T{hour:02}:{minute:02}:{second:02}Z")
}
