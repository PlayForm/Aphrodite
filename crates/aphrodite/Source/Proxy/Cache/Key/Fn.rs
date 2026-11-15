//! Derive the response-cache key from a request body.
//!
//! The key must be stable across processes and cheap to compute, and must
//! include the API key so two credentials never share a cached completion.
//! Returns `None` when the body cannot be keyed (unparseable, or a shape the
//! cache must not collapse) - a false key would serve one user's answer to
//! another.

use crate::Proxy::Cache::Hash::Fn::Fn as Fnv1a64;

/// Compute a cache key from a Chat Completions request body: hash(api_key +
/// model + messages). Uses FNV-1a (deterministic across restarts, unlike
/// DefaultHasher). Includes api_key to prevent cross-user cache collision.
/// Returns None if the body can't be parsed as JSON or lacks model/messages.
pub(crate) fn Fn(body:&[u8], APIKey:&str) -> Option<u64> {
	let v:serde_json::Value = serde_json::from_slice(body).ok()?;
	// F3: never cache a streamed request - the cached entry is a single
	// buffered JSON body, replayed with `Content-Type: application/json`,
	// which is nothing like an SSE stream a `"stream": true` client expects.
	if v.get("stream").and_then(|s| s.as_bool()).unwrap_or(false) {
		return None;
	}
	// `Model`/`messages` must both be present for this to be a valid,
	// cacheable chat-completion request.
	v.get("model")?.as_str()?;
	v.get("messages")?;
	// F3: the key used to hash only api_key+model+messages, so two requests
	// differing solely in `tools`, `tool_choice`, `temperature`, `top_p`,
	// `n`, or `response_format` collided and got served each other's cached
	// response. Include every field that changes what a valid response can
	// look like, in a fixed, canonical order (serde_json's key order from
	// `v` itself is not guaranteed stable across equivalent requests).
	let mut Parts:Vec<u8> = Vec::new();
	Parts.extend_from_slice(APIKey.as_bytes());
	for (Label, Value) in [
		("model", v.get("model")),
		("messages", v.get("messages")),
		("tools", v.get("tools")),
		("tool_choice", v.get("tool_choice")),
		("temperature", v.get("temperature")),
		("top_p", v.get("top_p")),
		("n", v.get("n")),
		("response_format", v.get("response_format")),
	] {
		Parts.push(b':');
		Parts.extend_from_slice(Label.as_bytes());
		Parts.push(b'=');
		if let Some(Value) = Value {
			Parts.extend_from_slice(serde_json::to_string(Value).ok()?.as_bytes());
		}
	}
	// FNV-1a 64-bit hash - deterministic across process restarts
	Some(Fnv1a64(&Parts))
}
