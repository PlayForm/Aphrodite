//! `Resolve` tests - the precedence chain, one knob family at a time.

use super::Guard::CwdGuard;
use crate::ConfigurationLoader::Configuration::Struct::Struct as Configuration;

#[test]
fn TestDefaults() {
	let cfg = Configuration::default();
	assert_eq!(cfg.GetU64("NONEXISTENT", "compression", "threshold", 42), 42);
	assert!(cfg.GetBool("NONEXISTENT", "compression", "enabled", true));
	assert_eq!(cfg.GetString("NONEXISTENT", "defaults", "model", "gpt-4o"), "gpt-4o");
}

#[test]
fn TestOverride() {
	let mut cfg = Configuration::default();
	cfg.SetOverride("APHRODITE_ENGINE_THRESHOLD_PCT", "90");
	assert_eq!(
		cfg.GetU64("APHRODITE_ENGINE_THRESHOLD_PCT", "compression", "engine_threshold_pct", 45),
		90
	);
}

// ── Fix 21 (inspection): `GetBool` must accept uppercase/mixed-case
// `TRUE`/`True`/`tRuE` and `1` through BOTH the overrides map and the
// environment path - the old exact lowercase comparison silently
// resolved them to false. Only `true`/`1` (after ASCII-lowercasing)
// count as true; `YES`/`on`/`0`/`FALSE` stay false. ──
#[test]
fn TestGetBoolOverrideCaseInsensitive() {
	let mut cfg = Configuration::default();
	[
		("TRUE", true),
		("True", true),
		("tRuE", true),
		("1", true),
		("YES", false),
		("on", false),
		("0", false),
		("FALSE", false),
	]
	.into_iter()
	.for_each(|(Value, Expected)| {
		cfg.SetOverride("APHRODITE_TEST_BOOL_CASE", Value);
		assert_eq!(
			cfg.GetBool("APHRODITE_TEST_BOOL_CASE", "compression", "enabled", true),
			Expected,
			"override value {Value:?} must resolve to {Expected}"
		);
	});
}

#[test]
fn TestGetBoolEnvironmentCaseInsensitive() {
	// Env-mutating tests serialize on CWD_GUARD (same rationale as the
	// directives tests in `Apply`).
	let _G = CwdGuard.get_or_init(|| std::sync::Mutex::new(())).lock().unwrap();
	const Key:&str = "APHRODITE_TEST_BOOL_CASE_ENV";
	// Hermetic (Rule 5): back up any prior value, remove it, set per
	// case, then remove and restore - a stray value must never leak
	// into this suite or later runs.
	let Backup = std::env::var(Key).ok();
	unsafe { std::env::remove_var(Key) };

	let cfg = Configuration::default();
	[
		("TRUE", true),
		("True", true),
		("tRuE", true),
		("1", true),
		("YES", false),
		("on", false),
		("0", false),
		("FALSE", false),
	]
	.into_iter()
	.for_each(|(Value, Expected)| {
		unsafe { std::env::set_var(Key, Value) };
		assert_eq!(
			cfg.GetBool(Key, "compression", "enabled", true),
			Expected,
			"env value {Value:?} must resolve to {Expected}"
		);
	});

	unsafe { std::env::remove_var(Key) };
	match Backup {
		Some(v) => {
			unsafe { std::env::set_var(Key, v) };
		},
		None => {},
	}
}
