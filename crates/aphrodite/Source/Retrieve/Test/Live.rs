//! The live-container contract: chunked progress on a partial retrieve, the
//! exact original content and `Complete` on the final resolve, the write-back
//! of accumulated progress, and degradation when the envelope is malformed.

use std::sync::Arc;

use axum::{
	extract::State,
	response::{IntoResponse, Json},
};

use crate::{
	Proxy::State::Struct::Struct as AppState,
	Retrieve::{Handle::Fn::Fn as Handle, Request::Struct::Struct as Request},
};

// ── Live containers (pre-Rust port of _hooks/live.py): the retrieve
// path serves container content progressively - chunk count /
// bytes-arrived surfaced on partial retrieves, full content + complete
// status on the final resolve (limit: 0). Non-container entries keep
// the pre-live response shape (no `Live` field). ──

// Seed an AppState inline store exactly as `Hook::transform_live_container`
// stores one: round-trip hash + meta envelope + per-chunk entries.
fn Seed(state:&AppState, hash:&str, Content:&str) -> usize {
	let Chunks:Vec<&str> = crate::Hook::Live::Chunk::Policy::Struct::Struct::new(crate::Hook::Live::Chunk::Const::Size)
		.expect("the shipped chunk target is non-zero")
		.Chunks(Content)
		.collect();
	{
		let mut Map = state.InlineCCR.lock().unwrap();
		Map.put(hash.to_string(), Content.to_string());
		let meta = serde_json::json!({
			"live": true,
			"type": "terminal",
			"size": Content.len(),
			"chunks": Chunks.len(),
			"chunk_size": crate::Hook::Live::Chunk::Const::Size,
			"bytes_arrived": 0,
		});
		Map.put(crate::Hook::Live::Key::Meta::Fn::Fn(hash), meta.to_string());
		Chunks.iter().enumerate().for_each(|(i, c)| {
			Map.put(crate::Hook::Live::Key::Chunk::Fn::Fn(hash, i), (*c).to_string());
		});
	}
	Chunks.len()
}

// Contract: a partial retrieve of a live container returns a chunked
// window with progress surfaced (chunk count, bytes arrived, not
// complete); the final resolve (limit: 0) returns the exact original
// content and marks the container complete.
#[test]
fn TestRetrieveLiveContainerChunkedProgressThenFullContent() {
	let state = crate::Proxy::Test::Support::State::Fn::WithCCR();
	let hash = "livehash_abc123";
	let Content = (0..40)
		.map(|i| format!("container line {i:02} - payload content\n"))
		.collect::<String>();
	let Count = Seed(&state, hash, &Content);
	assert!(Count >= 2, "test content must span multiple chunks");
	let state = Arc::new(state);

	async fn Decode(Response:axum::response::Response) -> serde_json::Value {
		let bytes = axum::body::to_bytes(Response.into_body(), usize::MAX).await.unwrap();
		serde_json::from_slice(&bytes).unwrap()
	}

	tokio::runtime::Runtime::new().unwrap().block_on(async {
		// 1. Partial retrieve → chunked progress, not complete.
		let Response = Handle(
			State(state.clone()),
			Json(Request { hash:Some(hash.to_string()), Query:None, Offset:0, Limit:2 }),
		)
		.await
		.into_response();
		let body = Decode(Response).await;
		assert_eq!(body["found"], true);
		assert_eq!(body["truncated"], true, "a windowed live retrieve must be flagged truncated");
		let Progress = &body["live"];
		assert_eq!(Progress["chunks"].as_u64().unwrap() as usize, Count);
		assert_eq!(Progress["bytes_total"].as_u64().unwrap() as usize, Content.len());
		assert_eq!(Progress["complete"], false, "a partial retrieve must not report completion");
		let Arrived = Progress["bytes_arrived"].as_u64().unwrap() as usize;
		assert!(
			Arrived > 0 && Arrived < Content.len(),
			"partial retrieve must advance bytes arrived: {Arrived}"
		);

		// 2. Final resolve (limit: 0) → full content, complete.
		let Response = Handle(
			State(state.clone()),
			Json(Request { hash:Some(hash.to_string()), Query:None, Offset:0, Limit:0 }),
		)
		.await
		.into_response();
		let body = Decode(Response).await;
		assert_eq!(body["found"], true);
		assert_eq!(body["content"], Content, "final resolve must return the exact original content");
		assert_eq!(body["truncated"], false);
		let Progress = &body["live"];
		assert_eq!(Progress["complete"], true, "final resolve must mark the container complete");
		assert_eq!(Progress["bytes_arrived"].as_u64().unwrap() as usize, Content.len());

		// The envelope's progress was written back for later retrieves.
		let Stored = state
			.InlineCCR
			.lock()
			.unwrap()
			.get(&crate::Hook::Live::Key::Meta::Fn::Fn(hash))
			.cloned()
			.unwrap();
		assert!(
			Stored.contains(&format!("\"bytes_arrived\":{}", Content.len())),
			"meta write-back must persist the accumulated progress: {Stored}"
		);
	});
}

// Contract: a plain (non-container) entry resolves exactly as before -
// no `Live` field appears in the response (byte-identical shape).
#[test]
fn TestRetrievePlainEntryHasNoLiveField() {
	let state = crate::Proxy::Test::Support::State::Fn::WithCCR();
	{
		let mut Map = state.InlineCCR.lock().unwrap();
		Map.put("plain1".to_string(), "plain content".to_string());
	}
	let state = Arc::new(state);

	async fn Decode(Response:axum::response::Response) -> serde_json::Value {
		let bytes = axum::body::to_bytes(Response.into_body(), usize::MAX).await.unwrap();
		serde_json::from_slice(&bytes).unwrap()
	}

	tokio::runtime::Runtime::new().unwrap().block_on(async {
		let Response = Handle(
			State(state.clone()),
			Json(Request { hash:Some("plain1".into()), Query:None, Offset:0, Limit:0 }),
		)
		.await
		.into_response();
		let body = Decode(Response).await;
		assert_eq!(body["content"], "plain content");
		assert_eq!(body["truncated"], false);
		assert!(
			body.get("live").is_none(),
			"non-container responses must not carry a live field"
		);
	});
}

// Contract: a stale/malformed envelope degrades to the plain entry
// (warn + degrade) instead of panicking or 500ing.
#[test]
fn TestRetrieveLiveContainerMalformedEnvelopeDegradesToPlain() {
	let state = crate::Proxy::Test::Support::State::Fn::WithCCR();
	{
		let mut Map = state.InlineCCR.lock().unwrap();
		Map.put("livehash_malformed".to_string(), "the plain content".to_string());
		Map.put(
			crate::Hook::Live::Key::Meta::Fn::Fn("livehash_malformed"),
			"this is not valid json {".to_string(),
		);
	}
	let state = Arc::new(state);

	async fn Decode(Response:axum::response::Response) -> serde_json::Value {
		let bytes = axum::body::to_bytes(Response.into_body(), usize::MAX).await.unwrap();
		serde_json::from_slice(&bytes).unwrap()
	}

	tokio::runtime::Runtime::new().unwrap().block_on(async {
		let Response = Handle(
			State(state.clone()),
			Json(Request { hash:Some("livehash_malformed".into()), Query:None, Offset:0, Limit:0 }),
		)
		.await
		.into_response();
		let body = Decode(Response).await;
		assert_eq!(body["found"], true);
		assert_eq!(
			body["content"], "the plain content",
			"malformed envelope must degrade to the plain entry"
		);
		assert!(
			body.get("live").is_none(),
			"degraded entries must not surface container progress"
		);
	});
}
