//! Compression thresholds: the env > TOML > default precedence, and the
//! `/reload` path that re-applies them to a live `AppState`.

use std::sync::atomic::Ordering;

use axum::{extract::State, response::IntoResponse};

use crate::{
	Configuration::Proxy::{Compression::Struct::Struct as CompressionConfiguration, Mode::Enum::Enum as ProxyMode},
	Proxy::{
		CCR::Reload::Fn::Fn as Reload,
		Cache::Hash::Fn::Fn as Fnv1a64,
		State::Struct::Struct as AppState,
		Test::Support::{Lock::Const::Lock, State::Fn::TestState},
		Threshold::{
			Const::{Cache, InitialExpand, Inline, Token},
			Resolve::Fn::Fn as ResolveThresholds,
		},
	},
};

#[test]
fn CacheCase() {
	// The fixture, not a second `AppState` literal: a hand-copied field set in a
	// test silently stops matching the real one (this copy had already drifted),
	// and every new field meant editing it twice.
	let state = AppState { Mode:ProxyMode::Cache, ..TestState() };

	assert_eq!(state.CompressThreshold(), Cache);
}

#[test]
fn Aphrodite() {
	let state = AppState { Mode:ProxyMode::Token, ..TestState() };
	assert_eq!(state.CompressThreshold(), Token);
}

// ── T15 (F2): TOML `[compression]` thresholds must actually be honored,
// not silently discarded in favor of the compiled-in consts. ──
#[test]
fn TomlOverrides() {
	let Comp = CompressionConfiguration {
		Trigger:None,
		First:None,
		Last:None,
		Minimum:None,
		Token:Some(512),
		Cache:Some(4096),
		Terminal:None,
		Inline:Some(2048),
		Expand:None,
		Limit:None,
		Body:None,
		Catalog:None,
		Poll:None,
		Multiplier:Some(5.0),
		Entries:None,
		Bytes:None,
	};
	let t = ResolveThresholds(Some(&Comp));
	assert_eq!(t.Cache, 4096);
	assert_eq!(t.Token, 512);
	assert_eq!(t.inline, 2048);
	assert_eq!(t.CodeMultiplier, 5.0);
}

#[test]
fn Defaults() {
	let t = ResolveThresholds(None);
	assert_eq!(t.Cache, Cache);
	assert_eq!(t.Token, Token);
	assert_eq!(t.inline, Inline);
	// F10: the corrected default - 2 only ever existed because
	// "3.0".parse::<usize>() silently failed.
	assert_eq!(t.CodeMultiplier, 3.0);
	// Pre-Rust port: auto-expand is off by default; the limit default
	// matches the Hermes-side `AphroditeState` default.
	assert!(!t.AutoExpand);
	assert_eq!(t.ExpandLimit, InitialExpand);
}

// ── Pre-Rust port: `[compression] auto_expand` / `Limit`
// must resolve from the TOML table into `ResolvedThresholds`, ready for
// `build_state` and `/reload` to apply. ──
#[test]
fn ExpandToml() {
	let Comp = CompressionConfiguration {
		Trigger:None,
		First:None,
		Last:None,
		Minimum:None,
		Token:None,
		Cache:None,
		Terminal:None,
		Inline:None,
		Expand:Some(true),
		Limit:Some(4096),
		Body:None,
		Catalog:None,
		Poll:None,
		Multiplier:None,
		Entries:None,
		Bytes:None,
	};
	let t = ResolveThresholds(Some(&Comp));
	assert!(t.AutoExpand);
	assert_eq!(t.ExpandLimit, 4096);
}

// ── T15 (F4): `/reload` must actually apply the new thresholds to the
// live `AppState`, not just re-parse and echo the file. ──
#[test]
fn ReloadThreshold() {
	let _Guard = Lock.lock().unwrap();
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
	let Response = Rt.block_on(Reload(State(state.clone()))).into_response();
	unsafe { std::env::remove_var("APHRODITE_CONFIG_PATH") };
	let _ = std::fs::remove_file(&path);

	assert_eq!(Response.status(), axum::http::StatusCode::OK);
	assert_eq!(state.TokenThreshold.load(Ordering::Relaxed), 999);
	assert_eq!(state.CacheThreshold.load(Ordering::Relaxed), 1234);
	assert_eq!(state.InlineThreshold.load(Ordering::Relaxed), 77);
	assert_eq!(state.CodeMultiplier.load(Ordering::Relaxed), 650);
}

// ── Pre-Rust port: `/reload` must also re-apply `AutoExpand` /
// `ExpandLimit` into the live `AppState`, exactly like the
// thresholds above - the compress paths consult these atomics on every
// response, so a hot-reload toggle must take effect immediately. ──
#[test]
fn ReloadExpand() {
	let _Guard = Lock.lock().unwrap();
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
	let Response = Rt.block_on(Reload(State(state.clone()))).into_response();
	unsafe { std::env::remove_var("APHRODITE_CONFIG_PATH") };
	let _ = std::fs::remove_file(&path);

	assert_eq!(Response.status(), axum::http::StatusCode::OK);
	assert!(state.AutoExpand.load(Ordering::Relaxed));
	assert_eq!(state.ExpandLimit.load(Ordering::Relaxed), 2048);
}
