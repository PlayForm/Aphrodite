//! The dev-only post-tool outcome line.
//!
//! Telemetry only: it writes one structured line to the session's debug file
//! and returns nothing. It must never change a hook's return value, and its
//! cost when the debug channel is off is zero - callers only invoke it after
//! `Debug::Log::Any::Fn::Fn` said yes.

use aphrodite::State::Struct::AphroditeState;

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

	// Optional payload fields in report order: an absent field is omitted
	// rather than written empty, so the line never claims a value it lacks.
	kv.extend(
		[
			r.get("reason")
				.and_then(|v| v.as_str())
				.map(|reason| ("reason", reason.to_string())),
			r.get("hash").and_then(|v| v.as_str()).map(|h| ("hash", h.to_string())),
			r.get("type").and_then(|v| v.as_str()).map(|t| ("type", t.to_string())),
			r.get("size").and_then(|v| v.as_u64()).map(|sz| ("size", sz.to_string())),
			r.get("chunks").and_then(|v| v.as_u64()).map(|ch| ("chunks", ch.to_string())),
			r.get("segments").and_then(|v| v.as_u64()).map(|n| ("segments", n.to_string())),
			r.get("auto_expanded").and_then(|v| v.as_bool()).unwrap_or(false).then(|| {
				let bytes = r
					.get("expanded")
					.and_then(|v| v.as_str())
					.map(|e| e.len().to_string())
					.unwrap_or_else(|| "true".to_string());

				("auto_expanded_bytes", bytes)
			}),
		]
		.into_iter()
		.flatten(),
	);

	crate::Debug::Log::Bridge::Fn::Fn(state, session, "post_tool", &kv);
}
