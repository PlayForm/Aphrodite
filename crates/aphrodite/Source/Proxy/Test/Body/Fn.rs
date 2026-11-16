//! Body handling: the request interpretation's delivery mode, the configured
//! size cap, and char-safe truncation.
//!
//! The delivery tests used to pin a byte-exact fast path that scanned for
//! `"stream":true` anywhere in the body. That scan was D3 - it could disagree
//! with the cache key's top-level parse - so the tests now pin the single
//! interpretation both consumers read, including the two bodies the old pair of
//! predicates classified differently.

use std::sync::Arc;

use axum::{body::Body, extract::FromRequest, http::StatusCode, response::IntoResponse};

use crate::Proxy::{
	Body::{Capped::Struct::Struct as CappedBody, Truncate::Fn::Fn as Truncate},
	Request::{
		Delivery::Enum::Enum as RequestDelivery,
		Route::Chat::Const::Path,
		Semantics::Fn::Fn as InterpretRequest,
	},
	Test::Support::State::Fn::{TestState, WithLimit},
};

/// The delivery mode one request body asks for.
fn Delivery(Body:&[u8]) -> RequestDelivery { InterpretRequest(Path, Body, "test-key").Delivery }

// ── 02-F2: the delivery mode decides which client (bounded vs
// no-total-timeout) a request goes out on - must read the top-level field real
// clients send, and fail closed (bounded client) on anything unparseable rather
// than accidentally going timeout-free for garbage input. ──
#[test]
fn TopLevel() {
	assert!(matches!(
		Delivery(br#"{"model":"gpt-4o","stream":true}"#),
		RequestDelivery::Streaming
	));
}

#[test]
fn Buffered() {
	assert!(matches!(Delivery(br#"{"model":"gpt-4o"}"#), RequestDelivery::Buffered));
	assert!(matches!(
		Delivery(br#"{"model":"gpt-4o","stream":false}"#),
		RequestDelivery::Buffered
	));
}

#[test]
fn InvalidJSON() {
	assert!(matches!(Delivery(b"not json"), RequestDelivery::Buffered));
}

#[test]
fn LargeBody() {
	// The flag sits after a large messages array; the parse is the authority, so
	// position in the body must not matter.
	let mut body = String::from("{\"model\":\"gpt-4o\",\"messages\":[");

	(0..500).for_each(|i| {
		(i > 0).then(|| body.push(','));
		body.push_str(&format!("{{\"role\":\"user\",\"content\":\"message {i}\"}}"));
	});
	body.push_str("],\"stream\":true}");

	assert!(matches!(Delivery(body.as_bytes()), RequestDelivery::Streaming));
}

#[test]
fn InString() {
	// The bytes `"stream": true` inside a message string must not decide
	// anything - only the parsed top-level field does.
	let body = br#"{"messages":[{"role":"user","content":"the docs say \"stream\": true"}]}"#;

	assert!(matches!(Delivery(body), RequestDelivery::Buffered));
}

// ── D3: one body, one classification. The old pair of predicates disagreed on
// both of these, so a request could be forwarded on the streaming client (no
// total timeout) AND be cached. ──
#[test]
fn NestedStream() {
	// `stream` at the top level is false; a nested object carries a true one.
	let body = br#"{"model":"m","stream":false,"meta":{"stream": true}}"#;

	assert!(matches!(Delivery(body), RequestDelivery::Buffered));
}

#[test]
fn StreamOptions() {
	// `stream_options` is a different field; its nested `"stream":true` is not
	// the top-level flag.
	let body = br#"{"model":"m","messages":[],"stream_options":{"stream":true}}"#;

	assert!(matches!(Delivery(body), RequestDelivery::Buffered));
}

// ── Issue #38 class: `&s[..80]` on a header value panics when byte 80 splits a
// UTF-8 char (multibyte header values in the dev-mode request log) - the
// char-boundary panic class the reporter saw in the binary's string table. ──
#[test]
fn Multibyte() {
	// All-2-byte chars: 100 bytes, byte 80 lands exactly on a boundary
	// (char 40) - must truncate to exactly 80 bytes.
	let Text = "é".repeat(50);
	let t = Truncate(&Text, 80);

	assert_eq!(t.len(), 80);
	assert!(Text.starts_with(t), "truncation must be a prefix");
	assert!(t.is_char_boundary(t.len()));

	// Mixed 1+2-byte chars so byte 80 lands INSIDE a char: property holds
	// regardless of where the walk lands.
	let Mixed = "aé".repeat(40); // 120 bytes, 3-byte units
	let T2 = Truncate(&Mixed, 80);

	assert!(T2.len() <= 80, "must never exceed the budget: {}", T2.len());
	assert!(T2.len() >= 77, "must not over-truncate (max 3-byte walk): {}", T2.len());
	assert!(Mixed.starts_with(T2), "truncation must be a prefix");
	assert!(T2.is_char_boundary(T2.len()));

	// Short input and pure-ASCII input pass through unchanged.
	assert_eq!(Truncate("abc", 80), "abc");
	assert_eq!(Truncate(&"x".repeat(100), 80).len(), 80);
}

#[tokio::test]
async fn OverLimit() {
	let State = Arc::new(WithLimit(16));
	let Request = axum::extract::Request::builder()
		.method("POST")
		.uri(format!("http://x{Path}"))
		.body(Body::from(vec![b'x'; 32]))
		.unwrap();
	let Rejection = CappedBody::from_request(Request, &State).await.unwrap_err();
	let response = Rejection.into_response();

	assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn UnderLimit() {
	let State = Arc::new(WithLimit(16));
	let body = vec![b'x'; 16];
	let Request = axum::extract::Request::builder()
		.method("POST")
		.uri(format!("http://x{Path}"))
		.body(Body::from(body.clone()))
		.unwrap();
	let Extracted = CappedBody::from_request(Request, &State).await.unwrap();

	assert_eq!(Extracted.0.as_ref(), body.as_slice());
}

#[tokio::test]
async fn NoCap() {
	// `max_request_body_size: None` (the default) must read the whole body -
	// byte-identical current behavior.
	let State = Arc::new(TestState());
	let body = vec![b'x'; 8192];
	let Request = axum::extract::Request::builder()
		.method("POST")
		.uri(format!("http://x{Path}"))
		.body(Body::from(body.clone()))
		.unwrap();
	let Extracted = CappedBody::from_request(Request, &State).await.unwrap();

	assert_eq!(Extracted.0.as_ref(), body.as_slice());
}
