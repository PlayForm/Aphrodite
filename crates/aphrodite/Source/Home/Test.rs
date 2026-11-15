//! The Home test battery: the precedence chain, tilde expansion, the derived
//! paths and the empty-value rule.
//!
//! Every test here mutates the process environment, so every test holds the
//! crate-wide [`EnvironmentGuard`] - without it a concurrent test's transient
//! `$HERMES_HOME` would silently rewrite the answer under test.

use std::path::PathBuf;

use crate::Home::{
	Configuration::Fn::Fn as ConfigurationPath,
	Database::Fn::Fn as CCRDatabasePath,
	Directive::Fn::Fn as DirectivesDirectory,
	Environment::Guard::Fn::Fn as EnvironmentGuard,
	Hermes::Fn::Fn as HermesHome,
	Plugin::Fn::Fn as PluginDirectory,
	Runtime::{Fn::Fn as RuntimeHome, Opt::Fn::Fn as RuntimeHomeOpt},
};

fn ClearAll() {
	unsafe {
		std::env::remove_var("APHRODITE_HOME");
		std::env::remove_var("HERMES_HOME");
		std::env::remove_var("HOME");
	}
}

#[test]
fn PrecedenceAphroditeHomeWins() {
	let _G = EnvironmentGuard();
	ClearAll();
	unsafe { std::env::set_var("APHRODITE_HOME", "/tmp/aph-override") };
	unsafe { std::env::set_var("HERMES_HOME", "/tmp/hermes-home") };
	unsafe { std::env::set_var("HOME", "/tmp/plain-home") };
	assert_eq!(RuntimeHome(), PathBuf::from("/tmp/aph-override"));
	assert_eq!(RuntimeHomeOpt(), Some(PathBuf::from("/tmp/aph-override")));
}

#[test]
fn PrecedenceHermesHomeBeatsHome() {
	// The issue-40 shape: HERMES_HOME set to something that is NOT
	// $HOME/.hermes - the runtime home must follow HERMES_HOME.
	let _G = EnvironmentGuard();
	ClearAll();
	unsafe { std::env::set_var("HERMES_HOME", "/opt/data") };
	unsafe { std::env::set_var("HOME", "/opt/data") };
	assert_eq!(RuntimeHome(), PathBuf::from("/opt/data/aphrodite"));
	assert_eq!(HermesHome(), PathBuf::from("/opt/data"));
	assert_eq!(PluginDirectory(), PathBuf::from("/opt/data/plugins/aphrodite"));
}

#[test]
fn PrecedenceHomeFallsBackToLegacy() {
	let _G = EnvironmentGuard();
	ClearAll();
	unsafe { std::env::set_var("HOME", "/tmp/plain-home") };
	assert_eq!(RuntimeHome(), PathBuf::from("/tmp/plain-home/.hermes/aphrodite"));
}

#[test]
fn TildeExpansion() {
	let _G = EnvironmentGuard();
	ClearAll();
	unsafe { std::env::set_var("HERMES_HOME", "~/scratch-home") };
	let Home = dirs::home_dir().expect("test machine has a home");
	assert_eq!(RuntimeHome(), Home.join("scratch-home").join("aphrodite"));
	unsafe { std::env::set_var("APHRODITE_HOME", "~") };
	assert_eq!(RuntimeHome(), Home);
}

#[test]
fn DerivedPathsFollowRuntimeHome() {
	let _G = EnvironmentGuard();
	ClearAll();
	unsafe { std::env::set_var("HERMES_HOME", "/opt/data") };
	let Root = PathBuf::from("/opt/data/aphrodite");
	assert_eq!(ConfigurationPath(), Root.join("aphrodite.toml"));
	assert_eq!(DirectivesDirectory(), Root.join("directives"));
	assert_eq!(CCRDatabasePath(), Root.join("ccr.db"));
}

#[test]
fn EmptyEnvironmentValuesAreIgnored() {
	// Empty-string overrides must fall through to the next level, never
	// produce an empty path (the Python shim strips empty values the
	// same way).
	let _G = EnvironmentGuard();
	ClearAll();
	unsafe { std::env::set_var("APHRODITE_HOME", "") };
	unsafe { std::env::set_var("HERMES_HOME", "/tmp/hermes-home") };
	assert_eq!(RuntimeHome(), PathBuf::from("/tmp/hermes-home/aphrodite"));
	unsafe { std::env::set_var("HERMES_HOME", "") };
	unsafe { std::env::set_var("HOME", "/tmp/plain-home") };
	assert_eq!(RuntimeHome(), PathBuf::from("/tmp/plain-home/.hermes/aphrodite"));
}

#[test]
fn DegradedReturnsNoneAndDot() {
	let _G = EnvironmentGuard();
	ClearAll();
	// HOME/HERMES_HOME/APHRODITE_HOME cleared; dirs::home_dir() still
	// resolves on a real machine, so runtime_home() must keep working.
	assert!(RuntimeHome().is_absolute() || RuntimeHome() == PathBuf::from("."));
	assert_eq!(RuntimeHomeOpt().is_some(), dirs::home_dir().is_some());
	assert!(HermesHome().is_absolute() || HermesHome() == PathBuf::from("."));
}
