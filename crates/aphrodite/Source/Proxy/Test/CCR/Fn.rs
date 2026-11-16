//! `/ccr/create`: unavailability, a failed store write, and the response's wire
//! shape.

use axum::{extract::State, response::IntoResponse};
use bytes::Bytes;

use crate::Proxy::{
	CCR::Create::{Handle::Fn::Fn as Create, Response::Struct::Struct as Body},
	Test::Support::State::Fn::{FailingCCR, TestState},
};

#[test]
fn Shape() {
	// Pins the wire contract of POST /ccr/create: field names and values
	// as they actually serialize, not just struct-literal field access.
	let Response = Body {
		hash:"abc123".into(),
		Ratio:2.5,
		OriginalSize:100,
		CompressedSize:40,
		MarkerSize:40,
	};
	let v = serde_json::to_value(&Response).unwrap();
	assert_eq!(v["hash"], "abc123");
	assert_eq!(v["original_size"], 100);
	assert_eq!(v["compressed_size"], 40);
	assert_eq!(v["marker_size"], 40);
	assert!((v["token_savings_ratio"].as_f64().unwrap() - 2.5).abs() < 0.01);
	// Regression guard for bench_01/02 (F3): the field is
	// `Ratio`, never `compression_ratio`.
	assert!(v.get("compression_ratio").is_none());
}

// ── T5 (F14): /ccr/create must report unavailability, not a
// fabricated success, when CCR isn't enabled or the store write fails. ──
#[test]
fn Disabled() {
	let mut state = TestState();
	state.ccr = None;
	let state = std::sync::Arc::new(state);
	let Rt = tokio::runtime::Runtime::new().unwrap();
	let Response = Rt
		.block_on(Create(
			State(state),
			axum::http::HeaderMap::new(),
			Bytes::from_static(b"hello world"),
		))
		.into_response();
	assert_eq!(Response.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
}

#[test]
fn PutFails() {
	let state = std::sync::Arc::new(FailingCCR());
	let Rt = tokio::runtime::Runtime::new().unwrap();
	let Response = Rt
		.block_on(Create(
			State(state),
			axum::http::HeaderMap::new(),
			Bytes::from_static(b"hello world"),
		))
		.into_response();
	assert_eq!(Response.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
}
