//! Environment variables beating the file, and the file beating the default.

use super::super::{Guard::EnvironmentGuard, MultiConfigurationFromToml};

// ── T17 (F1): env vars must override TOML values in multi-proxy mode,
// matching every shipped TOML's own header comment ("Env vars
// (APHRODITE_*) override TOML values at runtime") - previously only the
// API-key chain and the two port vars actually did this. ──
#[test]
fn TestResolveEnvironmentOverridesTomlForAPIURLModelTtlDatabaseNotify() {
	let _G = EnvironmentGuard();
	[
		("APHRODITE_API_URL", "https://env-api.example.com"),
		("APHRODITE_MODEL", "env-model"),
		("APHRODITE_CCR_TTL", "42"),
		("APHRODITE_DB", "/tmp/env-ccr.db"),
		("APHRODITE_NOTIFY_URL", "https://env-notify.example.com"),
		("APHRODITE_NOTIFY_KEY", "env-notify-key"),
	]
	.into_iter()
	.for_each(|(k, v)| unsafe { std::env::set_var(k, v) });
	let Mc = MultiConfigurationFromToml(
		r#"
		[[proxies]]
		name = "token"
		mode = "token"
		api_key = "test-key"
		api_url = "https://toml-api.example.com"
		model = "toml-model"
		ccr_ttl_seconds = 111
		ccr_db_path = "/tmp/toml-ccr.db"
		notify_url = "https://toml-notify.example.com"
		notify_key = "toml-notify-key"
		"#,
	);
	let CLI = Mc.Resolve(&Mc.Proxies[0]).unwrap();
	[
		"APHRODITE_API_URL",
		"APHRODITE_MODEL",
		"APHRODITE_CCR_TTL",
		"APHRODITE_DB",
		"APHRODITE_NOTIFY_URL",
		"APHRODITE_NOTIFY_KEY",
	]
	.into_iter()
	.for_each(|k| unsafe { std::env::remove_var(k) });
	assert_eq!(CLI.APIURL, "https://env-api.example.com");
	assert_eq!(CLI.Model, "env-model");
	assert_eq!(CLI.CCRTtlSeconds, 42);
	assert_eq!(CLI.CCRDatabasePath.unwrap().to_string_lossy(), "/tmp/env-ccr.db");
	assert_eq!(CLI.NotifyURL.as_deref(), Some("https://env-notify.example.com"));
	assert_eq!(CLI.NotifyKey.as_deref(), Some("env-notify-key"));
}

#[test]
fn TestResolveFallsBackToTomlWhenEnvironmentUnset() {
	let _G = EnvironmentGuard();
	["APHRODITE_API_URL", "APHRODITE_MODEL", "APHRODITE_CCR_TTL", "APHRODITE_DB"]
		.into_iter()
		.for_each(|k| unsafe { std::env::remove_var(k) });
	let Mc = MultiConfigurationFromToml(
		r#"
		[[proxies]]
		name = "token"
		mode = "token"
		api_key = "test-key"
		api_url = "https://toml-api.example.com"
		model = "toml-model"
		ccr_ttl_seconds = 111
		"#,
	);
	let CLI = Mc.Resolve(&Mc.Proxies[0]).unwrap();
	assert_eq!(CLI.APIURL, "https://toml-api.example.com");
	assert_eq!(CLI.Model, "toml-model");
	assert_eq!(CLI.CCRTtlSeconds, 111);
}

/// Store retention caps: env > `[compression]` > shipped default, and the
/// default must be FINITE. An unbounded store is the defect the key exists
/// to fix, so "absent everywhere" resolving to unlimited would silently
/// reinstate it.
#[test]
fn TestResolveStoreRetentionCapsEnvironmentTomlDefaults() {
	let _G = EnvironmentGuard();
	["APHRODITE_STORE_MAX_ENTRIES", "APHRODITE_STORE_MAX_BYTES"]
		.into_iter()
		.for_each(|k| unsafe { std::env::remove_var(k) });

	let toml = r#"
		[compression]
		store_max_entries = 1234
		store_max_bytes = 5678

		[[proxies]]
		name = "token"
		mode = "token"
		api_key = "test-key"
		"#;

	let Mc = MultiConfigurationFromToml(toml);
	let CLI = Mc.Resolve(&Mc.Proxies[0]).unwrap();
	assert_eq!(CLI.StoreMaximumEntries, 1234);
	assert_eq!(CLI.StoreMaximumBytes, 5678);

	unsafe {
		std::env::set_var("APHRODITE_STORE_MAX_ENTRIES", "9");
		std::env::set_var("APHRODITE_STORE_MAX_BYTES", "10");
	}
	let Mc = MultiConfigurationFromToml(toml);
	let CLI = Mc.Resolve(&Mc.Proxies[0]).unwrap();
	unsafe {
		std::env::remove_var("APHRODITE_STORE_MAX_ENTRIES");
		std::env::remove_var("APHRODITE_STORE_MAX_BYTES");
	}
	assert_eq!(CLI.StoreMaximumEntries, 9);
	assert_eq!(CLI.StoreMaximumBytes, 10);

	let Mc = MultiConfigurationFromToml(
		r#"
		[[proxies]]
		name = "token"
		mode = "token"
		api_key = "test-key"
		"#,
	);
	let CLI = Mc.Resolve(&Mc.Proxies[0]).unwrap();
	assert_eq!(CLI.StoreMaximumEntries, 100_000);
	assert_eq!(CLI.StoreMaximumBytes, 536_870_912);
}
