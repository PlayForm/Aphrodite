//! SSE: detection, and the handler's streaming path staying unbuffered and
//! uncached.

use std::sync::Arc;

use axum::response::IntoResponse;

use crate::Proxy::{
	Cache::{Get::Fn::Fn as Get, Identity::Struct::Struct as CacheIdentity},
	Handle::Proxy::Fn::Fn as ProxyHandler,
	Request::Route::Chat::Const::Path,
	Test::Support::{
		State::Fn::TestState,
		Upstream::Fn::{HeaderValue, Spawn, TestRequest},
	},
	Upstream::Media::{Enum::Enum as UpstreamMedia, Fn::Fn as ClassifyMedia},
};

#[test]
fn Exact() {
	assert!(matches!(
		ClassifyMedia(Some(&HeaderValue("text/event-stream"))),
		UpstreamMedia::EventStream
	));
}

#[test]
fn Charset() {
	assert!(matches!(
		ClassifyMedia(Some(&HeaderValue("text/event-stream; charset=utf-8"))),
		UpstreamMedia::EventStream
	));
}

#[test]
fn Reject() {
	assert!(matches!(
		ClassifyMedia(Some(&HeaderValue("application/json"))),
		UpstreamMedia::Buffered
	));
	assert!(matches!(
		ClassifyMedia(Some(&HeaderValue("text/plain"))),
		UpstreamMedia::Buffered
	));
	// Not a prefix match on a superstring that merely contains the token.
	assert!(matches!(
		ClassifyMedia(Some(&HeaderValue("text/x-event-stream"))),
		UpstreamMedia::Buffered
	));
}

#[test]
fn Missing() {
	assert!(matches!(ClassifyMedia(None), UpstreamMedia::Buffered));
}

#[tokio::test]
async fn Streamed() {
	let Addr = Spawn(
		"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nConnection: close\r\n\r\ndata: \
		 {\"delta\":\"hi\"}\n\ndata: [DONE]\n\n",
	)
	.await;

	let mut State = TestState();
	State.APIURL = format!("http://{Addr}");
	let State = Arc::new(State);

	// Non-streaming request body (no "stream": true) - proves the SSE
	// path is driven by the upstream's Content-Type, not the request.
	let (s, m, p, h, b) = TestRequest(State.clone(), Path, r#"{"model":"x","messages":[]}"#);
	let response = ProxyHandler(s, m, p, h, b).await.into_response();

	assert_eq!(
		response.headers().get("X-Aphrodite-Streamed").map(|v| v.to_str().unwrap()),
		Some("true"),
		"SSE responses must be marked with X-Aphrodite-Streamed"
	);
	assert_eq!(
		response.headers().get("content-type").map(|v| v.to_str().unwrap()),
		Some("text/event-stream")
	);

	// No cache write: an SSE response must never populate the response cache
	// (F3's whole point - a cached entry is a single buffered JSON body, nothing
	// like a stream). The identity is derived the same way the request path
	// derives it, so this looks under exactly the key a hit would have used.
	let Identity = CacheIdentity::New(&serde_json::json!({"model":"x","messages":[]}), State.APIKey.Expose());

	assert!(
		Get(&State, &Identity, std::time::Instant::now()).is_none(),
		"SSE response must not be cached"
	);
}

#[tokio::test]
async fn Plain() {
	let Addr =
		Spawn("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nConnection: close\r\n\r\n{\"ok\":true}").await;

	let mut State = TestState();
	State.APIURL = format!("http://{Addr}");
	let (s, m, p, h, b) = TestRequest(Arc::new(State), "/v1/models", "");
	let response = ProxyHandler(s, m, p, h, b).await.into_response();

	assert!(
		response.headers().get("X-Aphrodite-Streamed").is_none(),
		"a plain JSON upstream response must not be marked as streamed"
	);
}
