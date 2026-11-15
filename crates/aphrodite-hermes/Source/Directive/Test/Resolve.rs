//! Directory-resolution tests: the precedence chain, and the one decision it
//! shares with the engine binary.
//!
//! Both tests mutate process-global env vars, so each holds the env guard for
//! its whole body - without it the assertions race every other test in the
//! binary.

use std::path::PathBuf;

use super::Guard;
use crate::Directive::Resolve;

#[test]
fn resolve_directives_dir_precedence() {
	let _g = Guard::Fn();
	unsafe { std::env::remove_var("APHRODITE_DIRECTIVES_DIR") };
	unsafe { std::env::remove_var("APHRODITE_HOME") };
	unsafe { std::env::remove_var("HERMES_HOME") };
	unsafe { std::env::remove_var("HOME") };

	// No override at all -> the shared runtime-home resolution (the
	// platform user home) - no warnings, never fails (issue 40: the
	// resolution must keep working when $HOME alone is absent, because
	// the canonical home comes from $HERMES_HOME or the platform home).
	let (dir, warnings) = Resolve::Fn("");
	assert!(dir.to_string_lossy().ends_with("directives"));
	assert!(warnings.is_empty(), "the shared resolution must not warn: {warnings:?}");

	// $APHRODITE_HOME -> <home>/directives.
	unsafe { std::env::set_var("APHRODITE_HOME", "/tmp/aph-home") };
	let (dir, warnings) = Resolve::Fn("");
	assert_eq!(dir, PathBuf::from("/tmp/aph-home/directives"));
	assert!(warnings.is_empty());

	// $APHRODITE_DIRECTIVES_DIR (the loader's candidate 0) beats the home override.
	unsafe { std::env::set_var("APHRODITE_DIRECTIVES_DIR", "/tmp/aph-exact") };
	let (dir, _) = Resolve::Fn("");
	assert_eq!(dir, PathBuf::from("/tmp/aph-exact"));

	// An explicit home param beats every env override.
	let (dir, _) = Resolve::Fn("/tmp/param-home");
	assert_eq!(dir, PathBuf::from("/tmp/param-home/directives"));

	unsafe { std::env::remove_var("APHRODITE_DIRECTIVES_DIR") };
	unsafe { std::env::remove_var("APHRODITE_HOME") };
}

#[test]
fn resolve_directives_dir_follows_hermes_home() {
	// Issue 40: with HERMES_HOME set (Docker, profiles) the directives
	// must resolve under <hermes-home>/aphrodite - the same decision the
	// plugin shim makes - never under $HOME/.hermes/aphrodite.
	let _g = Guard::Fn();
	unsafe { std::env::remove_var("APHRODITE_DIRECTIVES_DIR") };
	unsafe { std::env::remove_var("APHRODITE_HOME") };
	unsafe { std::env::remove_var("HERMES_HOME") };
	unsafe { std::env::remove_var("HOME") };

	unsafe { std::env::set_var("HERMES_HOME", "/opt/data") };
	unsafe { std::env::set_var("HOME", "/opt/data") };
	let (dir, warnings) = Resolve::Fn("");
	assert_eq!(dir, PathBuf::from("/opt/data/aphrodite/directives"));
	assert!(warnings.is_empty());

	// $HOME alone (no HERMES_HOME) keeps the legacy location.
	unsafe { std::env::remove_var("HERMES_HOME") };
	unsafe { std::env::set_var("HOME", "/tmp/plain-home") };
	let (dir, _) = Resolve::Fn("");
	assert_eq!(dir, PathBuf::from("/tmp/plain-home/.hermes/aphrodite/directives"));

	unsafe { std::env::remove_var("HOME") };
}
