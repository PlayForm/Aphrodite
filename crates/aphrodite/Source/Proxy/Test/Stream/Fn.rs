//! SSE: detection, and the handler's streaming path staying unbuffered and
//! uncached.

use std::sync::Arc;

use axum::response::IntoResponse;

use crate::Proxy::Cache::Get::Fn::Fn as ResponseCacheGet;
use crate::Proxy::Cache::Key::Fn::Fn as CacheKeyFromBody;
use crate::Proxy::Handle::Proxy::Fn::Fn as ProxyHandler;
use crate::Proxy::Stream::Fn::Fn as IsSSE;
use crate::Proxy::Test::Support::Upstream::Fn::HeaderValue;
use crate::Proxy::Test::Support::Upstream::Fn::SpawnMockUpstream;
use crate::Proxy::Test::Support::Upstream::Fn::TestRequest;
use crate::Proxy::Test::Support::State::Fn::TestState;
use crate::Proxy::Threshold::Const::ChatCompletionsPath;

#[test]
fn TestIsSSEExactMatch() {
	assert!(IsSSE(Some(&HeaderValue("text/event-stream"))));
}

#[test]
fn TestIsSSEPrefixMatchWithCharset() {
	assert!(IsSSE(Some(&HeaderValue("text/event-stream; charset=utf-8"))));
}

#[test]
fn TestIsSSERejectsNonSSEContentTypes() {
	assert!(!IsSSE(Some(&HeaderValue("application/json"))));
	assert!(!IsSSE(Some(&HeaderValue("text/plain"))));
	// Not a prefix match on a superstring that merely contains the token.
	assert!(!IsSSE(Some(&HeaderValue("text/x-event-stream"))));
}

#[test]
fn TestIsSSEMissingHeaderIsFalse() {
	assert!(!IsSSE(None));
}

#[tokio::test]
async fn TestSSEResponseIsStreamedWithHeaderAndNotCached() {
	let Addr = SpawnMockUpstream(
		"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nConnection: close\r\n\r\ndata: \
		 {\"delta\":\"hi\"}\n\ndata: [DONE]\n\n",
	)
	.await;

	let mut State = TestState();
	State.APIURL = format!("http://{Addr}");
	let State = Arc::new(State);

	// Non-streaming request body (no "stream": true) - proves the SSE
	// path is driven by the upstream's Content-Type, not the request.
	let (s, m, p, h, b) = TestRequest(State.clone(), ChatCompletionsPath, r#"{"model":"x","messages":[]}"#);
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

	// No cache write: an SSE response must never populate the response
	// cache (F3's whole point - a cached entry is a single buffered JSON
	// body, nothing like a stream).
	let CacheKey = CacheKeyFromBody(r#"{"model":"x","messages":[]}"#.as_bytes(), State.APIKey.Expose());
	if let Some(Ck) = CacheKey {
		assert!(ResponseCacheGet(&State, Ck).is_none(), "SSE response must not be cached");
	}
}

#[tokio::test]
async fn TestNonSSEResponseIsNotStreamed() {
	let Addr = SpawnMockUpstream(
		"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nConnection: close\r\n\r\n{\"ok\":true}",
	)
	.await;

	let mut State = TestState();
	State.APIURL = format!("http://{Addr}");
	let (s, m, p, h, b) = TestRequest(Arc::new(State), "/v1/models", "");
	let response = ProxyHandler(s, m, p, h, b).await.into_response();

	assert!(
		response.headers().get("X-Aphrodite-Streamed").is_none(),
		"a plain JSON upstream response must not be marked as streamed"
	);
}
