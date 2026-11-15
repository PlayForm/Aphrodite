//! Port resolution: a malformed override falls back LOUDLY, a valid one wins.

use crate::Health::Port::Fn::{DEFAULT_CACHE_PORT, DEFAULT_TOKEN_PORT, configured_ports};

/// Serializes tests that touch process-global env vars
/// (`APHRODITE_CACHE_PORT`/`APHRODITE_TOKEN_PORT`).
fn env_guard() -> std::sync::MutexGuard<'static, ()> {
	static G:std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
	G.get_or_init(|| std::sync::Mutex::new(()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner)
}

// ── T9 (F11/F15): a malformed port env var must fall back to the
// default, not propagate a parse failure or silently pick something
// else - and it must not panic. ──
#[test]
fn test_configured_ports_falls_back_on_malformed_value() {
	let _g = env_guard();
	unsafe { std::env::set_var("APHRODITE_CACHE_PORT", "not-a-port") };
	unsafe { std::env::remove_var("APHRODITE_TOKEN_PORT") };
	let (cache, token) = configured_ports();
	unsafe { std::env::remove_var("APHRODITE_CACHE_PORT") };
	assert_eq!(cache, DEFAULT_CACHE_PORT);
	assert_eq!(token, DEFAULT_TOKEN_PORT);
}

#[test]
fn test_configured_ports_honors_valid_override() {
	let _g = env_guard();
	unsafe { std::env::set_var("APHRODITE_CACHE_PORT", "19797") };
	let (cache, _token) = configured_ports();
	unsafe { std::env::remove_var("APHRODITE_CACHE_PORT") };
	assert_eq!(cache, 19797);
}
