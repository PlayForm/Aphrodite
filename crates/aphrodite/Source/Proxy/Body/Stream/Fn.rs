//! Does this request body ask for a streamed (SSE) response?
//!
//! Checked *before* the upstream call so the handler can pick the unbounded
//! stream client: the upstream `Content-Type` is unknown until headers arrive,
//! by which point a bounded client's timeout is already ticking against the
//! whole response. A byte-exact fast path scans for `"stream":true` first; the
//! key's quotes are part of the pattern, so an escaped mention inside a string
//! value can never match.

/// 02-F2: does this request body ask for a streamed (SSE) response? Checked
/// before sending so `ProxyHandler` can pick `state.stream_client` (no
/// total timeout) instead of the bounded `state.client` - the upstream
/// `Content-Type` isn't known until headers come back, by which point a
/// bounded client's timeout is already ticking against the whole response.
///
/// W2-4: a full serde parse on every request just to read one boolean is
/// wasteful, so there is a byte-exact fast path first: scan for the two
/// canonical byte patterns `"stream":true` (no spaces) and `"stream": true`
/// (one space) - the key's quotes are part of the pattern, so an
/// escaped-quote mention inside a string value (`\"stream\": true`) can
/// never match: the backslash breaks the contiguous bytes, and valid JSON
/// never contains an unescaped `"` inside a string. Returning `true` only
/// on an exact match keeps false positives impossible; the parse below
/// stays the authority for every other spelling and falls through to it
/// on any non-match.
pub(crate) fn Fn(body:&[u8]) -> bool {
	let Direct =
		body.windows(13).any(|w| w == b"\"stream\":true") || body.windows(14).any(|w| w == b"\"stream\": true");

	match Direct {
		true => true,
		false => {
			serde_json::from_slice::<serde_json::Value>(body)
				.ok()
				.and_then(|v| v.get("stream").and_then(|s| s.as_bool()))
				.unwrap_or(false)
		},
	}
}
