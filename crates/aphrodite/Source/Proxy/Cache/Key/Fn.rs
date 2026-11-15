//! Derive the response-cache key from an already-parsed request body.
//!
//! The key must be stable across processes and cheap to compute, and must
//! include the API key so two credentials never share a cached completion.
//! Returns `None` when the body cannot be keyed (a shape the cache must not
//! collapse) - a false key would serve one user's answer to another.
//!
//! The parsed value is an argument rather than being parsed here: the request
//! is interpreted once (`Request::Semantics::Fn`), and re-parsing the same
//! bytes to answer a second question is exactly how one body came to be
//! classified two ways. The bytes fed to the digest are unchanged.

use crate::Proxy::Cache::Hash::Fn::Fn as Fnv1a64;

/// Compute a cache key from a parsed Chat Completions request body:
/// hash(api_key + model + messages + every field that changes the answer).
///
/// FNV-1a, deterministic across restarts (unlike `DefaultHasher`). The field
/// list is the cache format: adding a field invalidates nothing, removing or
/// reordering one aliases every entry already stored.
pub(crate) fn Fn(Body:&serde_json::Value, APIKey:&str) -> Option<u64> {
	// `Model`/`messages` must both be present for this to be a valid,
	// cacheable chat-completion request.
	Body.get("model")?.as_str()?;
	Body.get("messages")?;

	// F3: the key used to hash only api_key+model+messages, so two requests
	// differing solely in `tools`, `tool_choice`, `temperature`, `top_p`,
	// `n`, or `response_format` collided and got served each other's cached
	// response. Every field that changes what a valid response can look like
	// is included, in a fixed, canonical order (serde_json's key order from
	// `Body` itself is not guaranteed stable across equivalent requests).
	let mut Parts:Vec<u8> = Vec::new();
	Parts.extend_from_slice(APIKey.as_bytes());

	[
		("model", Body.get("model")),
		("messages", Body.get("messages")),
		("tools", Body.get("tools")),
		("tool_choice", Body.get("tool_choice")),
		("temperature", Body.get("temperature")),
		("top_p", Body.get("top_p")),
		("n", Body.get("n")),
		("response_format", Body.get("response_format")),
	]
	.into_iter()
	.try_for_each(|(Label, Value)| -> Option<()> {
		Parts.push(b':');
		Parts.extend_from_slice(Label.as_bytes());
		Parts.push(b'=');

		// `transpose` keeps the fail-closed semantics: a value that cannot be
		// serialised yields no key at all rather than a key that silently omits
		// the field it failed on.
		let Text = Value.map(serde_json::to_string).transpose().ok()?;

		let _ = Text.inspect(|Text| Parts.extend_from_slice(Text.as_bytes()));

		Some(())
	})?;

	// FNV-1a 64-bit hash - deterministic across process restarts
	Some(Fnv1a64(&Parts))
}
