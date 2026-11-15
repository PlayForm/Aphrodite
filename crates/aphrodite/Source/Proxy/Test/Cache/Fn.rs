//! The response cache: key stability, credential scoping, TTL expiry, and the
//! FNV vectors.

use std::time::Instant;

use bytes::Bytes;
use serde_json::Value;

use crate::Proxy::{
	Cache::{
		Entry::Struct::Struct as CacheEntry,
		Get::Fn::Fn as ResponseCacheGet,
		Hash::Fn::Fn as Fnv1a64,
		Key::Fn::Fn as CacheKeyFromBody,
	},
	Request::{
		Body::Enum::Enum as RequestBody,
		Cacheability::Enum::Enum as RequestCacheability,
		Delivery::Enum::Enum as RequestDelivery,
		Semantics::Fn::Fn as InterpretRequest,
	},
	Test::Support::State::Fn::TestState,
	Threshold::Const::ChatCompletionsPath,
};

// ── T3: cache_key_from_body / fnv1a_64 ────────────────────────
#[test]
fn TestCacheKeyFromBodyDeterministic() {
	let body:Value = serde_json::from_str(r#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}]}"#).unwrap();
	let K1 = CacheKeyFromBody(&body, "key-a");
	let K2 = CacheKeyFromBody(&body, "key-a");
	assert!(K1.is_some());
	assert_eq!(K1, K2);
}

#[test]
fn TestCacheKeyFromBodyDiffersByAPIKey() {
	let body:Value = serde_json::from_str(r#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}]}"#).unwrap();
	let K1 = CacheKeyFromBody(&body, "key-a");
	let K2 = CacheKeyFromBody(&body, "key-b");
	assert_ne!(K1, K2);
}

#[test]
fn TestCacheKeyFromBodyNoneOnUnkeyableBody() {
	// A body with no `model`/`messages` is valid JSON but not keyable. Bytes
	// that are not JSON at all never reach this function: `Semantics::Fn`
	// classifies them `Opaque` before a key is asked for.
	assert_eq!(CacheKeyFromBody(&Value::Null, "key"), None);
	assert_eq!(CacheKeyFromBody(&serde_json::json!({}), "key"), None); // missing model/messages
}

// ── T8 (F3): the cache key must cover the request parameters that
// change what a valid response can look like, and streamed requests
// must never be cached at all. ──
#[test]
fn TestCacheKeyFromBodyDiffersByToolsAndTemperature() {
	let Base:Value = serde_json::from_str(r#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}]}"#).unwrap();
	let WithTools:Value = serde_json::from_str(
		r#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}],"tools":[{"type":"function"}]}"#,
	)
	.unwrap();
	let WithTemp:Value =
		serde_json::from_str(r#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}],"temperature":0.7}"#)
			.unwrap();
	let KBase = CacheKeyFromBody(&Base, "key");
	let KTools = CacheKeyFromBody(&WithTools, "key");
	let KTemp = CacheKeyFromBody(&WithTemp, "key");
	assert!(KBase.is_some());
	assert_ne!(KBase, KTools, "differing `tools` must produce a different cache key");
	assert_ne!(KBase, KTemp, "differing `temperature` must produce a different cache key");
	assert_ne!(KTools, KTemp);
}

// ── D3: one interpretation. The streaming decision and the cache decision are
// read off the same parsed top-level field, so a body cannot be forwarded as a
// stream and cached as a buffered completion. ──
#[test]
fn TestSemanticsStreamingRequestIsUncacheable() {
	let Streamed = r#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}],"stream":true}"#;
	let Semantics = InterpretRequest(ChatCompletionsPath, Streamed.as_bytes(), "key");

	assert!(matches!(Semantics.Delivery, RequestDelivery::Streaming));
	assert!(matches!(Semantics.Cacheability, RequestCacheability::Uncacheable));
}

#[test]
fn TestSemanticsNestedStreamFlagDoesNotStream() {
	// The byte scan this replaced matched `"stream":true` at ANY depth, so a
	// buffered request that merely mentioned the word was forwarded on the
	// streaming client (no total timeout) while the key path still cached it.
	let Body = r#"{"model":"m","stream":false,"messages":[],"meta":{"stream":true}}"#;
	let Semantics = InterpretRequest(ChatCompletionsPath, Body.as_bytes(), "key");

	assert!(matches!(Semantics.Delivery, RequestDelivery::Buffered));
	assert!(matches!(Semantics.Cacheability, RequestCacheability::Cacheable { .. }));
}

#[test]
fn TestSemanticsOpaqueBodyIsBufferedAndUncacheable() {
	// The unknown-body path is preserved: not JSON, forwarded unchanged,
	// buffered, never keyed.
	let Semantics = InterpretRequest(ChatCompletionsPath, b"not json", "key");

	assert!(matches!(Semantics.Body, RequestBody::Opaque));
	assert!(matches!(Semantics.Delivery, RequestDelivery::Buffered));
	assert!(matches!(Semantics.Cacheability, RequestCacheability::Uncacheable));
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
		.put(42, CacheEntry { StoredAt:Instant::now(), Body:Bytes::from_static(b"cached") });
	std::thread::sleep(std::time::Duration::from_millis(20));
	assert_eq!(
		ResponseCacheGet(&State, 42, Instant::now()),
		None,
		"expired entry must not be returned"
	);
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
		.put(7, CacheEntry { StoredAt:Instant::now(), Body:Bytes::from_static(b"cached") });
	assert_eq!(ResponseCacheGet(&State, 7, Instant::now()), Some(Bytes::from_static(b"cached")));
}

#[test]
fn TestFnv1a64KnownVectors() {
	// FNV-1a 64-bit offset basis is the hash of the empty input.
	assert_eq!(Fnv1a64(b""), 14695981039346656037);
	// Two different inputs should (overwhelmingly likely) hash differently.
	assert_ne!(Fnv1a64(b"a"), Fnv1a64(b"b"));
}
