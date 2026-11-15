//! `EnvironmentBool` - one truthiness rule for boolean env vars everywhere.

use crate::Configuration::{Environment::Bool::Fn::EnvironmentBool, Test::Guard::EnvironmentGuard};

// ── T10 (F12): one truthiness rule for boolean env vars everywhere. ──
#[test]
fn TestEnvironmentBoolTrueValuesCaseInsensitive() {
	let _G = EnvironmentGuard();
	for v in ["1", "true", "TRUE", "True"] {
		unsafe { std::env::set_var("APHRODITE_TEST_BOOL", v) };
		assert!(EnvironmentBool("APHRODITE_TEST_BOOL"), "{v:?} should be true");
	}
	unsafe { std::env::remove_var("APHRODITE_TEST_BOOL") };
}

#[test]
fn TestEnvironmentBoolFalseValues() {
	let _G = EnvironmentGuard();
	for v in ["0", "false", "yes", ""] {
		unsafe { std::env::set_var("APHRODITE_TEST_BOOL", v) };
		assert!(!EnvironmentBool("APHRODITE_TEST_BOOL"), "{v:?} should be false");
	}
	unsafe { std::env::remove_var("APHRODITE_TEST_BOOL") };
	assert!(!EnvironmentBool("APHRODITE_TEST_BOOL"), "absent should be false");
}
