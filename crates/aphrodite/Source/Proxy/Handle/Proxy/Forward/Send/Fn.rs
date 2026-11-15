//! One attempt: build the upstream request and send it.
//!
//! Header handling is the contract here, and every exclusion below is a defect
//! that was already paid for:
//!
//! - `content-type`/`accept` are forced by the caller, so forwarding the
//!   client's own values too would append duplicate headers (reqwest's
//!   `header()` appends, it does not replace) and strict upstreams reject a
//!   duplicated `Content-Type` (F5/F18).
//! - `accept-encoding` is stripped entirely (F5): this client is built without
//!   gzip/brotli auto-decompression, so forwarding a client's
//!   `Accept-Encoding: gzip` gets a compressed body back that this proxy cannot
//!   decode, fails to JSON-parse (compression silently skipped), and returns to
//!   the caller as binary garbage labelled `application/json`.
//! - `host`, `authorization` and `content-length` are recomputed: the upstream
//!   host differs from the inbound one, the credential is the proxy's, and the
//!   body is re-encoded.
//! - `x-aphrodite-*` is this proxy's own vocabulary and must not reach an
//!   upstream that would try to interpret it.
//!
//! `HeaderName` is already lowercase by construction, so the comparisons are
//! against `Key.as_str()` directly - the old `.to_lowercase()` allocated on
//! every header of every attempt to reach the same answer.

use axum::http::{HeaderMap, Method};
use bytes::Bytes;
use reqwest::Client as HttpClient;

/// Send `Body` to `Url` with the forwarded headers, once.
pub(crate) async fn Fn(
	Client:&HttpClient,
	Method:&Method,
	Url:&str,
	Headers:&HeaderMap,
	APIKey:&str,
	Body:Bytes,
) -> Result<reqwest::Response, reqwest::Error> {
	let Request = Client
		.request(Method.clone(), Url)
		.header("Content-Type", "application/json; charset=utf-8")
		.header("Accept", "application/json")
		.header("Authorization", format!("Bearer {APIKey}"));

	Headers
		.iter()
		.filter(|(Key, _)| Forwardable(Key.as_str()))
		.fold(Request, |Request, (Key, Value)| Request.header(Key, Value))
		.body(Body)
		.send()
		.await
}

/// May this inbound header be forwarded to the upstream?
fn Forwardable(Name:&str) -> bool {
	!matches!(
		Name,
		"host" | "authorization" | "content-length" | "content-type" | "accept" | "accept-encoding"
	) && !Name.starts_with("x-aphrodite-")
}
