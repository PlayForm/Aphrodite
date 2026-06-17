#![allow(
	non_snake_case,
	non_camel_case_types,
	non_upper_case_globals,
	dead_code,
	unused_imports,
	unused_variables,
	unused_assignments
)]

//! # aphrodite build.rs
//!
//! Embeds version, git commit hash, build timestamp, target triple, rustc version,
//! and build profile as compile-time environment variables (`APHRODITE_*`) for
//! runtime display in startup logs, health checks, and version endpoints.
//!
//! CI builds rename the binary per target:
//!   aphrodite-linux-amd64    (x86_64-unknown-linux-gnu)
//!   aphrodite-macos-arm64    (aarch64-apple-darwin)
//!   aphrodite-macos-amd64    (x86_64-apple-darwin)

use serde::Deserialize;
use std::env;
use std::process::Command;

#[derive(Deserialize)]
struct Toml {
	package: Package,
}

#[derive(Deserialize)]
struct Package {
	version: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("cargo:rerun-if-changed=Cargo.toml");
	println!("cargo:rerun-if-changed=build.rs");
	println!("cargo:rerun-if-changed=src/");

	// Version from Cargo.toml (explicit, supplementing Cargo's built-in CARGO_PKG_VERSION)
	println!(
		"cargo:rustc-env=APHRODITE_VERSION={}",
		toml::from_str::<Toml>(&std::fs::read_to_string("Cargo.toml").expect("Cannot read Cargo.toml"))
			.expect("Cannot parse Cargo.toml")
			.package
			.version
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

	// Build timestamp (UTC ISO 8601)
	if let Ok(output) = Command::new("date").args(["-u", "+%Y-%m-%dT%H:%M:%SZ"]).output() {
		if output.status.success() {
			let ts = String::from_utf8_lossy(&output.stdout).trim().to_string();
			if !ts.is_empty() {
				println!("cargo:rustc-env=APHRODITE_BUILD_DATE={}", ts);
			}
		}
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
