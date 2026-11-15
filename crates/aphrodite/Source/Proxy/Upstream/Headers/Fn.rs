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
	Builder:axum::http::response::Builder,
	UpstreamHeaders:&reqwest::header::HeaderMap,
) -> axum::http::response::Builder {
	const Skip:&[&str] = &[
		"content-length",
		"content-type",
		"transfer-encoding",
		"connection",
		"keep-alive",
	];

	UpstreamHeaders
		.iter()
		.filter(|(name, _)| !Skip.contains(&name.as_str()))
		.fold(Builder, |Builder, (name, Value)| {
			match axum::http::HeaderValue::from_bytes(Value.as_bytes()) {
				Ok(v) => Builder.header(name.as_str(), v),
				Err(_) => Builder,
			}
		})
}
