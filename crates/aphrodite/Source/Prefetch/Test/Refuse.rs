//! The refusal guard's proof: every arm of `RefuseReason` must refuse, and
//! must refuse for the stated reason rather than by accident.
//!
//! These are security tests, not I/O tests. A change that makes prefetch load
//! a `.env`, an `auth.json`, a `~/.ssh` key, a Hermes-home secret or a
//! directory has removed the guard `Maintain/SECURITY.md` names, and one of
//! these must fail.

use crate::{Prefetch::Files::Fn::PrefetchFiles, State::AphroditeState};

// ── Teknium1 review ask 1: prefetch applies the same guard as
// `AphroditeRetrieve` (`read_path_guarded` semantics, implemented here
// in the core crate): refuse `.env` / `auth.json` / `~/.ssh` /
// Hermes-home / non-file paths with a distinct `Refused` outcome. ──

#[test]
fn TestPrefetchRefusesDotEnvironment() {
	// Exact file name `.env` is refused even though it exists and is
	// small - prefetch must never absorb credentials.
	let Directory = std::env::temp_dir().join(format!("aphrodite_prefetch_dotenv_{}", std::process::id()));
	std::fs::create_dir_all(&Directory).unwrap();
	let path = Directory.join(".env");
	std::fs::write(&path, "SECRET=value\n").unwrap();

	let mut s = AphroditeState::default();
	let r = PrefetchFiles(&mut s, &[path.to_string_lossy().to_string()]);
	assert_eq!(r["refused"], 1, "dot-env must be refused: {r:?}");
	assert_eq!(r["loaded"], 0);
	assert_eq!(r["results"][0]["status"], "refused");
	assert!(r["results"][0]["reason"].as_str().unwrap().contains(".env"));

	let _ = std::fs::remove_dir_all(&Directory);
}

#[test]
fn TestPrefetchRefusesAuthJSON() {
	let Directory = std::env::temp_dir().join(format!("aphrodite_prefetch_auth_{}", std::process::id()));
	std::fs::create_dir_all(&Directory).unwrap();
	let path = Directory.join("auth.json");
	std::fs::write(&path, "{\"token\":\"x\"}\n").unwrap();

	let mut s = AphroditeState::default();
	let r = PrefetchFiles(&mut s, &[path.to_string_lossy().to_string()]);
	assert_eq!(r["refused"], 1, "auth.json must be refused: {r:?}");
	assert_eq!(r["loaded"], 0);
	assert_eq!(r["results"][0]["status"], "refused");
	assert!(r["results"][0]["reason"].as_str().unwrap().contains("auth.json"));

	let _ = std::fs::remove_dir_all(&Directory);
}

#[test]
fn TestPrefetchRefusesSsh() {
	// Refusal is by `~/.ssh` prefix, so the file need not exist.
	let Home = dirs::home_dir().expect("test machine has a home");
	let Abs = Home.join(".ssh").join("id_rsa");
	let mut s = AphroditeState::default();
	let r = PrefetchFiles(&mut s, &[Abs.to_string_lossy().to_string()]);
	assert_eq!(r["refused"], 1, "~/.ssh path must be refused: {r:?}");
	assert_eq!(r["results"][0]["status"], "refused");

	// The literal tilde form must be refused too.
	let R2 = PrefetchFiles(&mut s, &["~/.ssh/id_rsa".to_string()]);
	assert_eq!(R2["refused"], 1, "tilde ~/.ssh path must be refused: {R2:?}");
}

#[test]
fn TestPrefetchRefusesHermesHome() {
	// Hold the SHARED env guard (crate::Home::Environment::Guard::Fn::Fn - one mutex for
	// every env-mutating test) for the WHOLE test, and clear HERMES_HOME
	// first: the default-home refusal must never observe a concurrent
	// test's transient override (that would silently turn `Refused` into
	// `Missing`). Default Hermes home (`~/.hermes`): refusal is by
	// prefix, no file needed on disk.
	let _G = crate::Home::Environment::Guard::Fn::Fn();
	let Prior = std::env::var_os("HERMES_HOME");
	unsafe { std::env::remove_var("HERMES_HOME") };
	let Home = dirs::home_dir().expect("test machine has a home");
	let p = Home.join(".hermes").join("aphrodite").join("tokens.json");
	let mut s = AphroditeState::default();
	let r = PrefetchFiles(&mut s, &[p.to_string_lossy().to_string()]);
	assert_eq!(r["refused"], 1, "~/.hermes path must be refused: {r:?}");
	assert_eq!(r["results"][0]["status"], "refused");

	// `$HERMES_HOME` override must be honored.
	let Directory = std::env::temp_dir().join(format!("aphrodite_prefetch_hh_{}", std::process::id()));
	std::fs::create_dir_all(&Directory).unwrap();
	std::fs::write(Directory.join("secrets.toml"), "token = \"x\"\n").unwrap();
	unsafe { std::env::set_var("HERMES_HOME", &Directory) };
	let R2 = PrefetchFiles(&mut s, &[Directory.join("secrets.toml").to_string_lossy().to_string()]);
	assert_eq!(R2["refused"], 1, "$HERMES_HOME path must be refused: {R2:?}");
	match Prior {
		Some(v) => unsafe { std::env::set_var("HERMES_HOME", v) },
		None => unsafe { std::env::remove_var("HERMES_HOME") },
	}
	let _ = std::fs::remove_dir_all(&Directory);
}

#[test]
fn TestPrefetchRefusesNonFileDirectory() {
	// An existing directory is a non-file path - refused, distinct from
	// a missing file (which stays `Missing`).
	let Directory = std::env::temp_dir().join(format!("aphrodite_prefetch_dir_{}", std::process::id()));
	std::fs::create_dir_all(&Directory).unwrap();

	let mut s = AphroditeState::default();
	let r = PrefetchFiles(&mut s, &[Directory.to_string_lossy().to_string()]);
	assert_eq!(r["refused"], 1, "directory must be refused: {r:?}");

	let _ = std::fs::remove_dir_all(&Directory);
}
