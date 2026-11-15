//! The response cache: key stability, credential scoping, TTL expiry, and the
//! FNV vectors.

use crate::Proxy::{
	Cache::{Get::Fn::Fn as ResponseCacheGet, Hash::Fn::Fn as Fnv1a64, Key::Fn::Fn as CacheKeyFromBody},
	Test::Support::TestState,
};

// ── T3: cache_key_from_body / fnv1a_64 ────────────────────────
#[test]
fn TestCacheKeyFromBodyDeterministic() {
	let body = br#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}]}"#;
	let K1 = CacheKeyFromBody(body, "key-a");
	let K2 = CacheKeyFromBody(body, "key-a");
	assert!(K1.is_some());
	assert_eq!(K1, K2);
}

#[test]
fn TestCacheKeyFromBodyDiffersByAPIKey() {
	let body = br#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}]}"#;
	let K1 = CacheKeyFromBody(body, "key-a");
	let K2 = CacheKeyFromBody(body, "key-b");
	assert_ne!(K1, K2);
}

#[test]
fn TestCacheKeyFromBodyNoneOnJunk() {
	assert_eq!(CacheKeyFromBody(b"not json", "key"), None);
	assert_eq!(CacheKeyFromBody(b"{}", "key"), None); // missing model/messages
}

// ── T8 (F3): the cache key must cover the request parameters that
// change what a valid response can look like, and streamed requests
// must never be cached at all. ──
#[test]
fn TestCacheKeyFromBodyDiffersByToolsAndTemperatureAndStream() {
	let Base = br#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}]}"#;
	let WithTools = br#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}],"tools":[{"type":"function"}]}"#;
	let WithTemp = br#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}],"temperature":0.7}"#;
	let KBase = CacheKeyFromBody(Base, "key");
	let KTools = CacheKeyFromBody(WithTools, "key");
	let KTemp = CacheKeyFromBody(WithTemp, "key");
	assert!(KBase.is_some());
	assert_ne!(KBase, KTools, "differing `tools` must produce a different cache key");
	assert_ne!(KBase, KTemp, "differing `temperature` must produce a different cache key");
	assert_ne!(KTools, KTemp);
}

#[test]
fn TestCacheKeyFromBodyNoneWhenStreaming() {
	let Streamed = br#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}],"stream":true}"#;
	assert_eq!(CacheKeyFromBody(Streamed, "key"), None);
}

// ── T1 remainder (F5, report 06): response_cache entries must expire
// on their own instead of being replayed forever. ──
#[test]
fn TestResponseCacheGetExpiresPastTtl() {
	let mut State = TestState();
	State.ResponseCacheTtl = std::time::Duration::from_millis(1);
	State
		.ResponseCache
		.lock()
		.unwrap()
		.put(42, (std::time::Instant::now(), b"cached".to_vec()));
	std::thread::sleep(std::time::Duration::from_millis(20));
	assert_eq!(ResponseCacheGet(&State, 42), None, "expired entry must not be returned");
	assert!(
		State.ResponseCache.lock().unwrap().peek(&42).is_none(),
		"expired entry must be evicted, not just skipped"
	);
}

#[test]
fn TestResponseCacheGetHitsWithinTtl() {
	let mut State = TestState();
	State.ResponseCacheTtl = std::time::Duration::from_secs(3600);
	State
		.ResponseCache
		.lock()
		.unwrap()
		.put(7, (std::time::Instant::now(), b"cached".to_vec()));
	assert_eq!(ResponseCacheGet(&State, 7), Some(b"cached".to_vec()));
}

#[test]
fn TestFnv1a64KnownVectors() {
	// FNV-1a 64-bit offset basis is the hash of the empty input.
	assert_eq!(Fnv1a64(b""), 14695981039346656037);
	// Two different inputs should (overwhelmingly likely) hash differently.
	assert_ne!(Fnv1a64(b"a"), Fnv1a64(b"b"));
}
