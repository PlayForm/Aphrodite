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
		Identity::Struct::Struct as CacheIdentity,
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

/// The identity of the one-message chat request the cache tests use.
fn Identity() -> CacheIdentity {
	let Body:Value = serde_json::from_str(r#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}]}"#).unwrap();

	CacheIdentity::New(&Body, "key")
}

// ── Cache identity: a digest to index by, canonical bytes for equality ──
#[test]
fn TestCacheIdentityDeterministic() {
	let body:Value = serde_json::from_str(r#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}]}"#).unwrap();
	let First = CacheIdentity::New(&body, "key-a");
	let Second = CacheIdentity::New(&body, "key-a");

	assert_eq!(First.Key(), Second.Key());
	assert!(First == Second, "the same request must produce the same identity");
}

#[test]
fn TestCacheIdentityDiffersByAPIKey() {
	let body:Value = serde_json::from_str(r#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}]}"#).unwrap();
	let First = CacheIdentity::New(&body, "key-a");
	let Second = CacheIdentity::New(&body, "key-b");

	assert_ne!(First.Key(), Second.Key());
	assert!(First != Second);
}

// ── The identity must represent the WHOLE response contract. The key it replaced
// hashed a hand-listed eight fields, so a request differing only in `max_tokens`
// or `stop` aliased one that did not, and was served the other's cached answer.
#[test]
fn TestCacheIdentityCoversFieldsOutsideTheOldKeyList() {
	let Base:Value = serde_json::from_str(r#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}]}"#).unwrap();
	let Capped:Value =
		serde_json::from_str(r#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}],"max_tokens":16}"#)
			.unwrap();
	let Stopped:Value =
		serde_json::from_str(r#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}],"stop":["\n\n"]}"#)
			.unwrap();

	let Base = CacheIdentity::New(&Base, "key");
	let Capped = CacheIdentity::New(&Capped, "key");
	let Stopped = CacheIdentity::New(&Stopped, "key");

	assert!(
		Base != Capped,
		"`max_tokens` changes the answer, so it must change the identity"
	);
	assert!(Base != Stopped, "`stop` changes the answer, so it must change the identity");
	assert!(Capped != Stopped);
}

// ── Canonical, not textual: the same contract in a different member order is the
// same request. Without this the identity would be a function of the client's
// serialiser rather than of what the client asked for.
#[test]
fn TestCacheIdentityIgnoresMemberOrder() {
	let Ordered:Value = serde_json::from_str(r#"{"model":"gpt-4","messages":[],"temperature":0.2}"#).unwrap();
	let Shuffled:Value = serde_json::from_str(r#"{"temperature":0.2,"messages":[],"model":"gpt-4"}"#).unwrap();
	let First = CacheIdentity::New(&Ordered, "key");
	let Second = CacheIdentity::New(&Shuffled, "key");

	assert_eq!(First.Key(), Second.Key());
	assert!(First == Second, "member order is not part of the request contract");
}

// ── T8 (F3): the identity must cover the request parameters that change what a
// valid response can look like, and streamed requests must never be cached at
// all. ──
#[test]
fn TestCacheIdentityDiffersByToolsAndTemperature() {
	let Base:Value = serde_json::from_str(r#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}]}"#).unwrap();
	let WithTools:Value = serde_json::from_str(
		r#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}],"tools":[{"type":"function"}]}"#,
	)
	.unwrap();
	let WithTemp:Value =
		serde_json::from_str(r#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}],"temperature":0.7}"#)
			.unwrap();
	let Base = CacheIdentity::New(&Base, "key");
	let WithTools = CacheIdentity::New(&WithTools, "key");
	let WithTemp = CacheIdentity::New(&WithTemp, "key");

	assert_ne!(
		Base.Key(),
		WithTools.Key(),
		"differing `tools` must produce a different identity"
	);
	assert_ne!(
		Base.Key(),
		WithTemp.Key(),
		"differing `temperature` must produce a different identity"
	);
	assert!(Base != WithTools);
	assert!(Base != WithTemp);
	assert!(WithTools != WithTemp);
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
	State.ResponseCache.lock().unwrap().put(
		Identity(),
		CacheEntry { StoredAt:Instant::now(), Body:Bytes::from_static(b"cached") },
	);
	std::thread::sleep(std::time::Duration::from_millis(20));
	assert_eq!(
		ResponseCacheGet(&State, &Identity(), Instant::now()),
		None,
		"expired entry must not be returned"
	);
	assert!(
		State.ResponseCache.lock().unwrap().peek(&Identity()).is_none(),
		"expired entry must be evicted, not just skipped"
	);
}

#[test]
fn TestResponseCacheGetHitsWithinTtl() {
	let mut State = TestState();
	State.ResponseCacheTtl = std::time::Duration::from_secs(3600);
	State.ResponseCache.lock().unwrap().put(
		Identity(),
		CacheEntry { StoredAt:Instant::now(), Body:Bytes::from_static(b"cached") },
	);
	assert_eq!(
		ResponseCacheGet(&State, &Identity(), Instant::now()),
		Some(Bytes::from_static(b"cached"))
	);
}

// ── The cache is keyed by the identity, so a stored answer is never served to a
// request that differs from it - not even one field outside the old key list. A
// digest match alone is an index hit, not equality evidence.
#[test]
fn TestResponseCacheDoesNotServeARequestThatDiffersOnlyInMaxTokens() {
	let mut State = TestState();
	State.ResponseCacheTtl = std::time::Duration::from_secs(3600);
	State.ResponseCache.lock().unwrap().put(
		Identity(),
		CacheEntry { StoredAt:Instant::now(), Body:Bytes::from_static(b"cached") },
	);

	let Capped:Value =
		serde_json::from_str(r#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}],"max_tokens":16}"#)
			.unwrap();
	let Capped = CacheIdentity::New(&Capped, "key");

	assert_eq!(
		ResponseCacheGet(&State, &Capped, Instant::now()),
		None,
		"a request with a different `max_tokens` must not be served another request's answer"
	);
}

#[test]
fn TestFnv1a64KnownVectors() {
	// FNV-1a 64-bit offset basis is the hash of the empty input.
	assert_eq!(Fnv1a64(b""), 14695981039346656037);
	// Two different inputs should (overwhelmingly likely) hash differently.
	assert_ne!(Fnv1a64(b"a"), Fnv1a64(b"b"));
}
