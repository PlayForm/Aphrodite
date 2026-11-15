//! `aphrodite_stats` self-diagnosis.
//!
//! A parse-failure recorded on the shared state must surface verbatim in
//! `aphrodite_stats`: the tracing warn is a no-op in the Hermes dylib, so this
//! field is the only guaranteed-visible signal that defaults are in effect.

use crate::Tool::Dispatch::Fn as dispatch;

#[test]
fn test_stats_surfaces_config_error() {
	let _g = crate::test_guard();
	let broken_path = "/tmp/aphrodite-cfg-broken-test.toml";

	// Simulate a found-but-broken aphrodite.toml (what
	// `Config::load().apply_compression()` records on the state).
	crate::with_shared(|s| s.ConfigurationError = Some(format!("{broken_path}: TOML parse error")));

	let r = dispatch("aphrodite_stats", "{}");
	assert_eq!(r["config_error"].as_str().unwrap(), format!("{broken_path}: TOML parse error"));
	assert!(
		r["auto_reload"].as_bool().unwrap_or(true) == false,
		"auto_reload must surface (default off): {:?}",
		r["auto_reload"]
	);

	// Restore the shared state so later tests are hermetic.
	crate::with_shared(|s| s.ConfigurationError = None);
	let r2 = dispatch("aphrodite_stats", "{}");
	assert!(
		r2["config_error"].is_null(),
		"config_error must be absent when config parsed fine"
	);
}
