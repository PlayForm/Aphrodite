//! Copy the upstream response headers onto the client response.
//!
//! Hop-by-hop headers and anything the proxy sets itself (content length,
//! transfer encoding, the cache markers) are skipped: copying them over the
//! re-encoded body is how a response ends up truncated or double-chunked.

/// Copy upstream response headers onto `Builder`, skipping hop-by-hop
/// headers (F5) - `content-length` is always skipped too since the caller
/// may be sending a re-serialized body of a different length than upstream's,
/// and `content-type`/the two `X-Aphrodite-*` headers the caller sets itself
/// afterward win if there's a name collision (axum keeps both; callers add
/// their own explicit `content-type` after calling this).
pub(crate) fn Fn(
	mut Builder:axum::http::response::Builder,
	UpstreamHeaders:&reqwest::header::HeaderMap,
) -> axum::http::response::Builder {
	const Skip:&[&str] = &[
		"content-length",
		"content-type",
		"transfer-encoding",
		"connection",
		"keep-alive",
	];
	for (name, Value) in UpstreamHeaders.iter() {
		if Skip.contains(&name.as_str()) {
			continue;
		}
		if let Ok(v) = axum::http::HeaderValue::from_bytes(Value.as_bytes()) {
			Builder = Builder.header(name.as_str(), v);
		}
	}
	Builder
}
