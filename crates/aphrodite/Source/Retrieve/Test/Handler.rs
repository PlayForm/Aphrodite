//! Handler-level contracts: hash normalisation and the poisoned-lock verdict.

use std::sync::Arc;

use axum::{
	extract::State,
	http::StatusCode,
	response::{IntoResponse, Json},
};

use crate::Retrieve::{Handle::Fn::Fn as Handle, Request::Struct::Struct as Request};

// ── T5 (F3): handle_retrieve must normalize the `hash` argument the
// same way `Resolve::One::Fn` already does - strip a `|type|size` marker-body
// suffix an LLM might echo back, and trim surrounding whitespace. Seeds
// `InlineCCR` directly (it's checked first, before any CCR backend
// round-trip) via the crate's shared `AppState` test fixture instead of
// duplicating its ~47-field literal.
#[test]
fn TestHandleRetrieveNormalizesPipeSuffixedAndWhitespaceHash() {
	let state = crate::Proxy::Test::Support::State::Fn::WithCCR();
	{
		let mut Map = state.InlineCCR.lock().unwrap();
		Map.put("abc123".to_string(), "<the real content>".to_string());
	}
	let state = Arc::new(state);

	async fn Decode(Response:axum::response::Response) -> serde_json::Value {
		let bytes = axum::body::to_bytes(Response.into_body(), usize::MAX).await.unwrap();
		serde_json::from_slice(&bytes).unwrap()
	}

	tokio::runtime::Runtime::new().unwrap().block_on(async {
		// Residue: a `for` whose body contains `.await` - an iterator adapter cannot
		// await, and this crate's `futures` dependency is optional.
		for Argument in ["abc123", "abc123|tool|1024", "  abc123  "] {
			let Response = Handle(
				State(state.clone()),
				Json(Request { hash:Some(Argument.to_string()), Query:None, Offset:0, Limit:0 }),
			)
			.await
			.into_response();
			let body = Decode(Response).await;
			assert_eq!(body["found"], true, "hash arg {Argument:?} must resolve: {body:?}");
			assert_eq!(body["content"], "<the real content>");
		}
	});
}

// ── Fix 20 (inspection): a poisoned inline_ccr lock must surface as an
// explicit 500 with a clear error - NOT silently degrade into a cache
// miss (which turned every request into a confusing 404). ──
#[test]
fn TestHandleRetrievePoisonedInlineCCRLockReturns500() {
	let state = crate::Proxy::Test::Support::State::Fn::WithCCR();
	// Poison the mutex: panic while holding the guard (contained via
	// catch_unwind so it stays inside this test).
	let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
		let _Guard = state.InlineCCR.lock().unwrap();
		panic!("intentional poison of inline_ccr");
	}));
	assert!(state.InlineCCR.is_poisoned(), "test precondition: lock must be poisoned");
	let state = Arc::new(state);

	tokio::runtime::Runtime::new().unwrap().block_on(async {
		let Response = Handle(
			State(state.clone()),
			Json(Request { hash:Some("abc123".into()), Query:None, Offset:0, Limit:0 }),
		)
		.await
		.into_response();
		assert_eq!(Response.status(), StatusCode::INTERNAL_SERVER_ERROR);
		let bytes = axum::body::to_bytes(Response.into_body(), usize::MAX).await.unwrap();
		let body:serde_json::Value = serde_json::from_slice(&bytes).unwrap();
		assert_eq!(body["found"], false);
		let Error = body["error"].as_str().unwrap();
		assert!(Error.contains("poisoned"), "error must identify the poisoned lock: {Error}");
	});
}
