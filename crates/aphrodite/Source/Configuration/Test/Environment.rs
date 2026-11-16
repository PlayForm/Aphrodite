//! `Environment::Bool::Fn` - one truthiness rule for boolean env vars everywhere.

use crate::Configuration::{Environment::Bool::Fn::Fn as Bool, Test::Guard::EnvironmentGuard};

// ── T10 (F12): one truthiness rule for boolean env vars everywhere. ──
#[test]
fn TestEnvironmentBoolTrueValuesCaseInsensitive() {
	let _G = EnvironmentGuard();
	["1", "true", "TRUE", "True"].into_iter().for_each(|v| {
		unsafe { std::env::set_var("APHRODITE_TEST_BOOL", v) };
		assert!(Bool("APHRODITE_TEST_BOOL"), "{v:?} should be true");
	});
	unsafe { std::env::remove_var("APHRODITE_TEST_BOOL") };
}

#[test]
fn TestEnvironmentBoolFalseValues() {
	let _G = EnvironmentGuard();
	["0", "false", "yes", ""].into_iter().for_each(|v| {
		unsafe { std::env::set_var("APHRODITE_TEST_BOOL", v) };
		assert!(!Bool("APHRODITE_TEST_BOOL"), "{v:?} should be false");
	});
	unsafe { std::env::remove_var("APHRODITE_TEST_BOOL") };
	assert!(!Bool("APHRODITE_TEST_BOOL"), "absent should be false");
}
