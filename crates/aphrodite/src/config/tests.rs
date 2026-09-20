use super::*;

/// Serializes tests that touch process-global env vars
/// (`APHRODITE_CACHE_PORT`/`APHRODITE_TOKEN_PORT`), since `cargo test`
/// runs this module's tests concurrently by default.
fn env_guard() -> std::sync::MutexGuard<'static, ()> {
	static G:std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
	G.get_or_init(|| std::sync::Mutex::new(()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner)
}

// ── T10 (F12): one truthiness rule for boolean env vars everywhere. ──
#[test]
fn test_env_bool_true_values_case_insensitive() {
	let _g = env_guard();
	for v in ["1", "true", "TRUE", "True"] {
		unsafe { std::env::set_var("APHRODITE_TEST_BOOL", v) };
		assert!(env_bool("APHRODITE_TEST_BOOL"), "{v:?} should be true");
	}
	unsafe { std::env::remove_var("APHRODITE_TEST_BOOL") };
}

#[test]
fn test_env_bool_false_values() {
	let _g = env_guard();
	for v in ["0", "false", "yes", ""] {
		unsafe { std::env::set_var("APHRODITE_TEST_BOOL", v) };
		assert!(!env_bool("APHRODITE_TEST_BOOL"), "{v:?} should be false");
	}
	unsafe { std::env::remove_var("APHRODITE_TEST_BOOL") };
	assert!(!env_bool("APHRODITE_TEST_BOOL"), "absent should be false");
}

fn multi_config_from_toml(toml_str:&str) -> MultiConfig { toml::from_str(toml_str).expect("valid test TOML") }

#[test]
fn test_resolve_default_ports_per_mode() {
	let _g = env_guard();
	unsafe { std::env::remove_var("APHRODITE_CACHE_PORT") };
	unsafe { std::env::remove_var("APHRODITE_TOKEN_PORT") };

	let mc = multi_config_from_toml(
		r#"
		[[proxies]]
		name = "cache"
		mode = "cache"
		api_key = "test-key"
		"#,
	);
	let cli = mc.resolve(&mc.proxies[0]).unwrap();
	assert_eq!(cli.listen.port(), 9797); // default listen, no override present

	let mc = multi_config_from_toml(
		r#"
		[[proxies]]
		name = "token"
		mode = "token"
		api_key = "test-key"
		"#,
	);
	let cli = mc.resolve(&mc.proxies[0]).unwrap();
	assert_eq!(cli.listen.port(), 9797); // still 9797: no explicit `listen` was set in the TOML
}

#[test]
fn test_resolve_explicit_port_override_via_env() {
	let _g = env_guard();
	unsafe { std::env::set_var("APHRODITE_CACHE_PORT", "19797") };
	let mc = multi_config_from_toml(
		r#"
		[[proxies]]
		name = "cache"
		mode = "cache"
		api_key = "test-key"
		listen = "127.0.0.1:9797"
		"#,
	);
	let cli = mc.resolve(&mc.proxies[0]).unwrap();
	assert_eq!(cli.listen.port(), 19797);
	unsafe { std::env::remove_var("APHRODITE_CACHE_PORT") };
}

// ── T17 (F1): env vars must override TOML values in multi-proxy mode,
// matching every shipped TOML's own header comment ("Env vars
// (APHRODITE_*) override TOML values at runtime") - previously only the
// API-key chain and the two port vars actually did this. ──
#[test]
fn test_resolve_env_overrides_toml_for_api_url_model_ttl_db_notify() {
	let _g = env_guard();
	for (k, v) in [
		("APHRODITE_API_URL", "https://env-api.example.com"),
		("APHRODITE_MODEL", "env-model"),
		("APHRODITE_CCR_TTL", "42"),
		("APHRODITE_DB", "/tmp/env-ccr.db"),
		("APHRODITE_NOTIFY_URL", "https://env-notify.example.com"),
		("APHRODITE_NOTIFY_KEY", "env-notify-key"),
	] {
		unsafe { std::env::set_var(k, v) };
	}
	let mc = multi_config_from_toml(
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
	let cli = mc.resolve(&mc.proxies[0]).unwrap();
	for k in [
		"APHRODITE_API_URL",
		"APHRODITE_MODEL",
		"APHRODITE_CCR_TTL",
		"APHRODITE_DB",
		"APHRODITE_NOTIFY_URL",
		"APHRODITE_NOTIFY_KEY",
	] {
		unsafe { std::env::remove_var(k) };
	}
	assert_eq!(cli.api_url, "https://env-api.example.com");
	assert_eq!(cli.model, "env-model");
	assert_eq!(cli.ccr_ttl_seconds, 42);
	assert_eq!(cli.ccr_db_path.unwrap().to_string_lossy(), "/tmp/env-ccr.db");
	assert_eq!(cli.notify_url.as_deref(), Some("https://env-notify.example.com"));
	assert_eq!(cli.notify_key.as_deref(), Some("env-notify-key"));
}

#[test]
fn test_resolve_falls_back_to_toml_when_env_unset() {
	let _g = env_guard();
	for k in ["APHRODITE_API_URL", "APHRODITE_MODEL", "APHRODITE_CCR_TTL", "APHRODITE_DB"] {
		unsafe { std::env::remove_var(k) };
	}
	let mc = multi_config_from_toml(
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
	let cli = mc.resolve(&mc.proxies[0]).unwrap();
	assert_eq!(cli.api_url, "https://toml-api.example.com");
	assert_eq!(cli.model, "toml-model");
	assert_eq!(cli.ccr_ttl_seconds, 111);
}

#[test]
fn test_resolve_invalid_mode_falls_back_to_token() {
	let mc = multi_config_from_toml(
		r#"
		[[proxies]]
		name = "weird"
		mode = "not_a_real_mode"
		api_key = "test-key"
		"#,
	);
	let cli = mc.resolve(&mc.proxies[0]).unwrap();
	assert!(matches!(cli.mode, ProxyMode::Token));
}

#[test]
fn test_resolve_missing_mode_falls_back_to_token() {
	let mc = multi_config_from_toml(
		r#"
		[[proxies]]
		name = "no_mode"
		api_key = "test-key"
		"#,
	);
	let cli = mc.resolve(&mc.proxies[0]).unwrap();
	assert!(matches!(cli.mode, ProxyMode::Token));
}

#[test]
fn test_resolve_timeout_clamped_to_600() {
	let mc = multi_config_from_toml(
		r#"
		[[proxies]]
		name = "slow"
		api_key = "test-key"
		timeout = 9999
		"#,
	);
	let cli = mc.resolve(&mc.proxies[0]).unwrap();
	assert_eq!(cli.timeout, 600);
}

#[test]
fn test_resolve_timeout_under_max_is_unchanged() {
	let mc = multi_config_from_toml(
		r#"
		[[proxies]]
		name = "normal"
		api_key = "test-key"
		timeout = 120
		"#,
	);
	let cli = mc.resolve(&mc.proxies[0]).unwrap();
	assert_eq!(cli.timeout, 120);
}

#[test]
fn test_resolve_missing_api_key_errors() {
	let _g = env_guard();
	unsafe { std::env::remove_var("APHRODITE_API_KEY") };
	unsafe { std::env::remove_var("DEEPSEEK_API_KEY") };
	unsafe { std::env::remove_var("HEADROOM_DEEPSEEK_KEY") };
	let mc = multi_config_from_toml(
		r#"
		[[proxies]]
		name = "no_key"
		"#,
	);
	let result = mc.resolve(&mc.proxies[0]);
	assert!(result.is_err());
}

#[test]
fn test_resolve_invalid_listen_address_errors() {
	let mc = multi_config_from_toml(
		r#"
		[[proxies]]
		name = "bad_listen"
		api_key = "test-key"
		listen = "not-an-address"
		"#,
	);
	let result = mc.resolve(&mc.proxies[0]);
	assert!(result.is_err());
}

#[test]
fn test_resolve_max_output_must_be_less_than_max_context() {
	let mc = multi_config_from_toml(
		r#"
		[[proxies]]
		name = "bad_budget"
		api_key = "test-key"
		max_context = 100
		max_output = 200
		"#,
	);
	let result = mc.resolve(&mc.proxies[0]);
	assert!(result.is_err());
}

#[test]
fn test_resolve_defaults_fill_in_missing_proxy_fields() {
	// api_url/model are env-overridable since T17 - guard against the
	// process-global env vars racing with other tests in this module.
	let _g = env_guard();
	unsafe { std::env::remove_var("APHRODITE_API_URL") };
	unsafe { std::env::remove_var("APHRODITE_MODEL") };
	let mc = multi_config_from_toml(
		r#"
		[defaults]
		api_key = "default-key"
		api_url = "https://default.example.com"
		model = "default-model-name"

		[[proxies]]
		name = "uses_defaults"
		"#,
	);
	let cli = mc.resolve(&mc.proxies[0]).unwrap();
	assert_eq!(cli.api_key, "default-key");
	assert_eq!(cli.api_url, "https://default.example.com");
	assert_eq!(cli.model, "default-model-name");
}

// ── Point 5 (feedback): a hook-only configuration that omits the
// `[proxies]` table entirely must parse, defaulting `proxies` to an
// empty Vec - not fail with `missing field 'proxies'`. ──
#[test]
fn test_multi_config_without_proxies_table_defaults_to_empty() {
	let mc = multi_config_from_toml(
		r#"
		[compression]
		engine_threshold_pct = 45
		"#,
	);
	assert!(
		mc.proxies.is_empty(),
		"missing [proxies] table must default to an empty Vec, got {} entries",
		mc.proxies.len()
	);
}

// ── Point 5 (feedback): an explicit empty `proxies = []` also parses. ──
#[test]
fn test_multi_config_explicit_empty_proxies() {
	let mc = multi_config_from_toml(
		r#"
		proxies = []
		"#,
	);
	assert!(mc.proxies.is_empty());
}
