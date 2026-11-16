//! Compression thresholds: the env > TOML > default precedence, and the
//! `/reload` path that re-applies them to a live `AppState`.

use std::sync::atomic::Ordering;

use axum::{extract::State, response::IntoResponse};

use crate::{
	Configuration::Proxy::{Compression::Struct::Struct as CompressionConfiguration, Mode::Enum::Enum as ProxyMode},
	Proxy::{
		CCR::Reload::Fn::Fn as HandleCCRReload,
		Cache::Hash::Fn::Fn as Fnv1a64,
		State::Struct::Struct as AppState,
		Test::Support::{Lock::Const::ReloadTestLock, State::Fn::TestState},
		Threshold::{
			Const::{AutoExpandLimitDefault, CacheCompressThreshold, InlineCCRThreshold, TokenCompressThreshold},
			Resolve::Fn::Fn as ResolveThresholds,
		},
	},
};

#[test]
fn TestCompressThresholdCache() {
	// The fixture, not a second `AppState` literal: a hand-copied field set in a
	// test silently stops matching the real one (this copy had already drifted),
	// and every new field meant editing it twice.
	let state = AppState { Mode:ProxyMode::Cache, ..TestState() };

	assert_eq!(state.CompressThreshold(), CacheCompressThreshold);
}

#[test]
fn TestCompressThresholdAphrodite() {
	let state = AppState { Mode:ProxyMode::Token, ..TestState() };
	assert_eq!(state.CompressThreshold(), TokenCompressThreshold);
}

// ── T15 (F2): TOML `[compression]` thresholds must actually be honored,
// not silently discarded in favor of the compiled-in consts. ──
#[test]
fn TestResolveThresholdsTomlOverridesDefaults() {
	let Comp = CompressionConfiguration {
		EngineThresholdPct:None,
		EngineProtectFirst:None,
		EngineProtectLast:None,
		EngineMinimumMsgs:None,
		ToolThresholdToken:Some(512),
		ToolThresholdCache:Some(4096),
		TerminalThreshold:None,
		InlineThreshold:Some(2048),
		AutoExpand:None,
		AutoExpandLimit:None,
		MaximumRequestBodySize:None,
		CatalogMode:None,
		ClassifierPoll:None,
		CodeMultiplier:Some(5.0),
		StoreMaximumEntries:None,
		StoreMaximumBytes:None,
	};
	let t = ResolveThresholds(Some(&Comp));
	assert_eq!(t.Cache, 4096);
	assert_eq!(t.Token, 512);
	assert_eq!(t.inline, 2048);
	assert_eq!(t.CodeMultiplier, 5.0);
}

#[test]
fn TestResolveThresholdsDefaultsWhenNoToml() {
	let t = ResolveThresholds(None);
	assert_eq!(t.Cache, CacheCompressThreshold);
	assert_eq!(t.Token, TokenCompressThreshold);
	assert_eq!(t.inline, InlineCCRThreshold);
	// F10: the corrected default - 2 only ever existed because
	// "3.0".parse::<usize>() silently failed.
	assert_eq!(t.CodeMultiplier, 3.0);
	// Pre-Rust port: auto-expand is off by default; the limit default
	// matches the Hermes-side `AphroditeState` default.
	assert!(!t.AutoExpand);
	assert_eq!(t.AutoExpandLimit, AutoExpandLimitDefault);
}

// ── Pre-Rust port: `[compression] auto_expand` / `AutoExpandLimit`
// must resolve from the TOML table into `ResolvedThresholds`, ready for
// `build_state` and `/reload` to apply. ──
#[test]
fn TestResolveThresholdsAutoExpandTomlEnabled() {
	let Comp = CompressionConfiguration {
		EngineThresholdPct:None,
		EngineProtectFirst:None,
		EngineProtectLast:None,
		EngineMinimumMsgs:None,
		ToolThresholdToken:None,
		ToolThresholdCache:None,
		TerminalThreshold:None,
		InlineThreshold:None,
		AutoExpand:Some(true),
		AutoExpandLimit:Some(4096),
		MaximumRequestBodySize:None,
		CatalogMode:None,
		ClassifierPoll:None,
		CodeMultiplier:None,
		StoreMaximumEntries:None,
		StoreMaximumBytes:None,
	};
	let t = ResolveThresholds(Some(&Comp));
	assert!(t.AutoExpand);
	assert_eq!(t.AutoExpandLimit, 4096);
}

// ── T15 (F4): `/reload` must actually apply the new thresholds to the
// live `AppState`, not just re-parse and echo the file. ──
#[test]
fn TestHandleCCRReloadAppliesThresholdsToState() {
	let _Guard = ReloadTestLock.lock().unwrap();
	let Directory = std::env::temp_dir();
	let path = Directory.join(format!(
		"aphrodite_reload_test_{}_{}.toml",
		std::process::id(),
		Fnv1a64(b"reload-test-salt")
	));
	std::fs::write(
		&path,
		r#"
[[proxies]]
name = "token"
mode = "token"

[compression]
tool_threshold_token = 999
tool_threshold_cache = 1234
inline_threshold = 77
code_multiplier = 6.5
"#,
	)
	.unwrap();

	// No other test in this crate reads/writes APHRODITE_CONFIG_PATH.
	unsafe { std::env::set_var("APHRODITE_CONFIG_PATH", &path) };
	let state = std::sync::Arc::new(TestState());
	let Rt = tokio::runtime::Runtime::new().unwrap();
	let Response = Rt.block_on(HandleCCRReload(State(state.clone()))).into_response();
	unsafe { std::env::remove_var("APHRODITE_CONFIG_PATH") };
	let _ = std::fs::remove_file(&path);

	assert_eq!(Response.status(), axum::http::StatusCode::OK);
	assert_eq!(state.TokenCompressThreshold.load(Ordering::Relaxed), 999);
	assert_eq!(state.CacheCompressThreshold.load(Ordering::Relaxed), 1234);
	assert_eq!(state.InlineCCRThreshold.load(Ordering::Relaxed), 77);
	assert_eq!(state.CodeMultiplierX100.load(Ordering::Relaxed), 650);
}

// ── Pre-Rust port: `/reload` must also re-apply `AutoExpand` /
// `AutoExpandLimit` into the live `AppState`, exactly like the
// thresholds above - the compress paths consult these atomics on every
// response, so a hot-reload toggle must take effect immediately. ──
#[test]
fn TestHandleCCRReloadAppliesAutoExpandToState() {
	let _Guard = ReloadTestLock.lock().unwrap();
	let Directory = std::env::temp_dir();
	let path = Directory.join(format!(
		"aphrodite_reload_auto_expand_test_{}_{}.toml",
		std::process::id(),
		Fnv1a64(b"auto-expand-reload-salt")
	));
	std::fs::write(
		&path,
		r#"
[[proxies]]
name = "token"
mode = "token"

[compression]
auto_expand = true
auto_expand_limit = 2048
"#,
	)
	.unwrap();

	// No other test in this crate reads/writes APHRODITE_CONFIG_PATH.
	unsafe { std::env::set_var("APHRODITE_CONFIG_PATH", &path) };
	let state = std::sync::Arc::new(TestState());
	let Rt = tokio::runtime::Runtime::new().unwrap();
	let Response = Rt.block_on(HandleCCRReload(State(state.clone()))).into_response();
	unsafe { std::env::remove_var("APHRODITE_CONFIG_PATH") };
	let _ = std::fs::remove_file(&path);

	assert_eq!(Response.status(), axum::http::StatusCode::OK);
	assert!(state.AutoExpand.load(Ordering::Relaxed));
	assert_eq!(state.AutoExpandLimit.load(Ordering::Relaxed), 2048);
}
