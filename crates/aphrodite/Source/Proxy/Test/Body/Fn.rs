//! Body handling: the stream flag's byte-exact fast path, the configured size
//! cap, and char-safe truncation.

use std::sync::Arc;

use axum::{body::Body, extract::FromRequest, http::StatusCode, response::IntoResponse};

use crate::Proxy::{
	Body::{
		Capped::Struct::Struct as CappedBody,
		Stream::Fn::Fn as BodyWantsStream,
		Truncate::Fn::Fn as TruncateCharSafe,
	},
	Test::Support::State::Fn::{TestState, TestStateWithMaximumBody},
	Threshold::Const::ChatCompletionsPath,
};

// ── 02-F2: `BodyWantsStream` decides which client (bounded vs
// no-total-timeout) a request goes out on - must match on the exact
// field real clients send, and fail closed (bounded client) on anything
// unparseable rather than accidentally going timeout-free for garbage
// input. ──
#[test]
fn TestBodyWantsStreamTrueWhenSet() {
	assert!(BodyWantsStream(br#"{"model":"gpt-4o","stream":true}"#));
}

#[test]
fn TestBodyWantsStreamFalseWhenAbsentOrFalse() {
	assert!(!BodyWantsStream(br#"{"model":"gpt-4o"}"#));
	assert!(!BodyWantsStream(br#"{"model":"gpt-4o","stream":false}"#));
}

#[test]
fn TestBodyWantsStreamFalseOnInvalidJSON() {
	assert!(!BodyWantsStream(b"not json"));
}

#[test]
fn TestBodyWantsStreamLargeBodyFlagAtEnd() {
	// W2-4 fast path: the flag sits after a large messages array, so a
	// byte scan (not the full serde parse) must find it.
	let mut body = String::from("{\"model\":\"gpt-4o\",\"messages\":[");
	for i in 0..500 {
		if i > 0 {
			body.push(',');
		}
		body.push_str(&format!("{{\"role\":\"user\",\"content\":\"message {i}\"}}"));
	}
	body.push_str("],\"stream\":true}");
	assert!(BodyWantsStream(body.as_bytes()));
}

#[test]
fn TestBodyWantsStreamFlagInMessageStringEscapedQuotes() {
	// W2-4: the pattern inside a message string has escaped quotes
	// (`\"stream\": true`), which insert backslash bytes and break the
	// contiguous byte pattern - the fast path must not fire, and the
	// parse (authority) returns false since `Stream` is not a key.
	let body = br#"{"messages":[{"role":"user","content":"the docs say \"stream\": true"}]}"#;
	assert!(!BodyWantsStream(body));
}

#[test]
fn TestBodyWantsStreamLargeBodyWithoutFlag() {
	// W2-4: large body with no `Stream` key anywhere - fast path misses
	// and the parse agrees.
	let mut body = String::from("{\"model\":\"gpt-4o\",\"messages\":[");
	for i in 0..500 {
		if i > 0 {
			body.push(',');
		}
		body.push_str(&format!("{{\"role\":\"user\",\"content\":\"message {i}\"}}"));
	}
	body.push_str("]}");
	assert!(!BodyWantsStream(body.as_bytes()));
}

// ── Issue #38 class: `&s[..80]` on a header value panics when byte 80
// splits a UTF-8 char (multibyte header values in the dev-mode request
// log) - the char-boundary panic class the reporter saw in the binary's
// string table. ──
#[test]
fn TestTruncateCharSafeNeverSplitsMultibyte() {
	// All-2-byte chars: 100 bytes, byte 80 lands exactly on a boundary
	// (char 40) - must truncate to exactly 80 bytes.
	let Multibyte = "é".repeat(50);
	let t = TruncateCharSafe(&Multibyte, 80);
	assert_eq!(t.len(), 80);
	assert!(Multibyte.starts_with(t), "truncation must be a prefix");
	assert!(t.is_char_boundary(t.len()));

	// Mixed 1+2-byte chars so byte 80 lands INSIDE a char: property
	// holds regardless of where the walk lands.
	let Mixed = "aé".repeat(40); // 120 bytes, 3-byte units
	let T2 = TruncateCharSafe(&Mixed, 80);
	assert!(T2.len() <= 80, "must never exceed the budget: {}", T2.len());
	assert!(T2.len() >= 77, "must not over-truncate (max 3-byte walk): {}", T2.len());
	assert!(Mixed.starts_with(T2), "truncation must be a prefix");
	assert!(T2.is_char_boundary(T2.len()));

	// Short input and pure-ASCII input pass through unchanged.
	assert_eq!(TruncateCharSafe("abc", 80), "abc");
	assert_eq!(TruncateCharSafe(&"x".repeat(100), 80).len(), 80);
}

#[tokio::test]
async fn TestBodyCapRejectsOverLimitWith413() {
	let State = Arc::new(TestStateWithMaximumBody(16));
	let Request = axum::extract::Request::builder()
		.method("POST")
		.uri(format!("http://x{ChatCompletionsPath}"))
		.body(Body::from(vec![b'x'; 32]))
		.unwrap();
	let Rejection = CappedBody::from_request(Request, &State).await.unwrap_err();
	let response = Rejection.into_response();
	assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn TestBodyCapAcceptsBodyAtOrUnderLimit() {
	let State = Arc::new(TestStateWithMaximumBody(16));
	let body = vec![b'x'; 16];
	let Request = axum::extract::Request::builder()
		.method("POST")
		.uri(format!("http://x{ChatCompletionsPath}"))
		.body(Body::from(body.clone()))
		.unwrap();
	let Extracted = CappedBody::from_request(Request, &State).await.unwrap();
	assert_eq!(Extracted.0.as_ref(), body.as_slice());
}

#[tokio::test]
async fn TestBodyCapNoneIsNoCap() {
	// `max_request_body_size: None` (the default) must read the whole
	// body - byte-identical current behavior.
	let State = Arc::new(TestState());
	let body = vec![b'x'; 8192];
	let Request = axum::extract::Request::builder()
		.method("POST")
		.uri(format!("http://x{ChatCompletionsPath}"))
		.body(Body::from(body.clone()))
		.unwrap();
	let Extracted = CappedBody::from_request(Request, &State).await.unwrap();
	assert_eq!(Extracted.0.as_ref(), body.as_slice());
}
