//! The dev-only post-tool outcome line.
//!
//! Telemetry only: it writes one structured line to the session's debug file
//! and returns nothing. It must never change a hook's return value, and its
//! cost when the debug channel is off is zero - callers only invoke it after
//! `Debug::Log::Any::Fn` said yes.

use aphrodite::State::AphroditeState;

/// DEV-ONLY: post-tool outcome line built from the transform payload
/// (marker emitted with hash+type+size, auto-expanded bytes, live container
/// chunks, chain-split segments, or a passthrough reason) + result size +
/// elapsed microseconds.
pub(crate) fn post_tool_log(
	state:&AphroditeState,
	session:&str,
	r:&serde_json::Value,
	result_bytes:usize,
	elapsed_us:u64,
) {
	let mut kv = vec![
		("result_bytes", result_bytes.to_string()),
		("elapsed_us", elapsed_us.to_string()),
		(
			"compressed",
			r.get("compressed").and_then(|v| v.as_bool()).unwrap_or(false).to_string(),
		),
	];
	if let Some(reason) = r.get("reason").and_then(|v| v.as_str()) {
		kv.push(("reason", reason.to_string()));
	}
	if let Some(h) = r.get("hash").and_then(|v| v.as_str()) {
		kv.push(("hash", h.to_string()));
	}
	if let Some(t) = r.get("type").and_then(|v| v.as_str()) {
		kv.push(("type", t.to_string()));
	}
	if let Some(sz) = r.get("size").and_then(|v| v.as_u64()) {
		kv.push(("size", sz.to_string()));
	}
	if let Some(ch) = r.get("chunks").and_then(|v| v.as_u64()) {
		kv.push(("chunks", ch.to_string()));
	}
	if let Some(n) = r.get("segments").and_then(|v| v.as_u64()) {
		kv.push(("segments", n.to_string()));
	}
	if r.get("auto_expanded").and_then(|v| v.as_bool()).unwrap_or(false) {
		let bytes = r
			.get("expanded")
			.and_then(|v| v.as_str())
			.map(|e| e.len().to_string())
			.unwrap_or_else(|| "true".to_string());
		kv.push(("auto_expanded_bytes", bytes));
	}
	crate::Debug::Log::Bridge::Fn(state, session, "post_tool", &kv);
}
