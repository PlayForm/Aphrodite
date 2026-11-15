//! //! Send the upstream request, with the connect-only retry policy.
//!
//! At most three attempts, and only for connect-phase failures: a post-send
//! timeout may already have been accepted upstream, and blindly retrying a
//! non-idempotent `POST /v1/chat/completions` risks double token billing. The
//! returned flag says whether the final failure was a timeout, because
//! `UpstreamTimeouts` counting every transport error was a metrics lie (F17).
//!
//! The body is converted to `Bytes` once, outside the loop, so each attempt
//! clones a refcount rather than the payload (bug 18-P9).

use std::sync::Arc;

use axum::http::Method;

use crate::Proxy::{Body::Stream::Fn::Fn as BodyWantsStream, State::Struct::Struct as AppState};

pub(crate) async fn Fn(
	State:&Arc<AppState>,
	Method:&Method,
	url:&str,
	Headers:&axum::http::HeaderMap,
	BodyVec:Vec<u8>,
) -> (anyhow::Result<reqwest::Response>, bool) {
	let mut UpstreamResult:anyhow::Result<reqwest::Response> = Err(anyhow::anyhow!("unreachable"));
	// F17: which counter the final error increments - `UpstreamTimeouts`
	// used to count every kind of transport failure (connection refused,
	// DNS failure, TLS error) as a "timeout", which is a real metrics lie
	// for anyone diagnosing outages from `/metrics`.
	let mut FinalErrorWasTimeout = false;
	// 02-F2: a `"stream": true` request goes out on `StreamClient` (no
	// total timeout), not the bounded `Client` - see its doc comment.
	let HttpClient = if BodyWantsStream(&BodyVec) { &State.StreamClient } else { &State.Client };
	// F10 fix (above) builds `BodyVec` once (one Vec alloc). Convert to
	// `Bytes` here - outside the retry loop - so each attempt clones in O(1)
	// (refcount) instead of copying the whole payload. The no-retry common
	// path pays a single buffer move, not the two full copies the prior
	// `body_vec.clone()` forced on every request (bug 18-P9: up to 4 copies
	// of a 1MB body under retry load).
	let BodyBytes = bytes::Bytes::from(BodyVec);
	// Retry loop: up to 3 attempts total with at most 2 retries - the
	// `attempt < 3` guard below means attempt 3 never retries, it records
	// the final failure instead. Only connect-phase failures retry (F17).
	for Attempt in 1..=3u32 {
		let Request = HttpClient
			.request(Method.clone(), url)
			.header("Content-Type", "application/json; charset=utf-8")
			.header("Accept", "application/json")
			.header("Authorization", format!("Bearer {}", State.APIKey.Expose()));
		let Request = Headers.iter().fold(Request, |Request, (Key, Value)| {
			let k = Key.as_str().to_lowercase();
			// Strip (F5/F18): `content-type`/`accept` are forced above already -
			// forwarding the client's own values too would append duplicate
			// headers (reqwest's `header()` appends, it doesn't replace), and
			// strict upstreams reject a duplicated `Content-Type`.
			// `accept-encoding` is stripped entirely (F5): this client is built
			// without gzip/brotli auto-decompression
			// (`Cargo.toml`'s reqwest features), so forwarding a client's
			// `Accept-Encoding: gzip` gets a compressed body back that this
			// proxy can't decode, fails to JSON-parse (compression silently
			// skipped), and returns to the caller as binary garbage labeled
			// `application/json`.
			match k != "host"
				&& k != "authorization"
				&& k != "content-length"
				&& k != "content-type"
				&& k != "accept"
				&& k != "accept-encoding"
				&& !k.starts_with("x-aphrodite-")
			{
				true => Request.header(Key, Value),
				false => Request,
			}
		});
		// `BodyBytes` is the `Bytes` built once before the loop (see
		// above); clone per attempt is O(1) refcount, never a full copy.
		match Request.body(BodyBytes.clone()).send().await {
			Ok(r) => {
				UpstreamResult = Ok(r);
				break;
			},
			Err(e) => {
				// F17: only retry connect-phase failures - the request never
				// left, so resending is safe. A post-send request TIMEOUT
				// (`e.is_timeout()` after the body was already transmitted)
				// may have been accepted by the upstream; blindly retrying a
				// non-idempotent `POST /v1/chat/completions` risks double
				// token billing, and combined with the 300s per-attempt
				// timeout, up to 3 attempts (at most 2 retries) could hold a
				// client for ~15 minutes. Non-connect errors now fail fast on
				// the first attempt instead.
				match Attempt < 3 && e.is_connect() {
					true => {
						let BaseMs = 100 * 2u64.pow(Attempt - 1);
						let Jitter = rand::random::<f64>() * 0.5 + 0.75; // 0.75x to 1.25x
						let Ms = (BaseMs as f64 * Jitter) as u64;
						tracing::warn!(Attempt, BackoffMs = Ms, "upstream retry after connect error: {}", e);
						tokio::time::sleep(std::time::Duration::from_millis(Ms)).await;
					},
					false => {
						FinalErrorWasTimeout = e.is_timeout();
						UpstreamResult =
							Err(anyhow::Error::new(e).context(format!("send the upstream request to {url}")));
						break;
					},
				}
			},
		}
	}
	(UpstreamResult, FinalErrorWasTimeout)
}
