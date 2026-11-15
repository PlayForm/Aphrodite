//! The `[aphrodite-debug ...]` line prepended to a compressed tool result.
//!
//! This is the only debug surface the MODEL sees: the flag is on, so instead of
//! retrieving the marker the session can read what the engine did inline. It is
//! therefore deliberately a single line of scalar fields - never the payload,
//! never a multi-line block that would compete with the marker.

use crate::Debug::Flag::Resolve::Fn::Fn as enabled_for;

/// Build a `[aphrodite-debug ...]` prefix line from a compression result
/// when the flag is on for `session`; `None` when off (no allocation on the
/// quiet path).
///
/// It is not a log sink - the line is returned, not written, because its
/// destination is the tool-result stream. Missing fields render as `?`/`0`
/// rather than being omitted, so the line has a stable shape to read.
pub(crate) fn Fn(r:&serde_json::Value, session:&str) -> Option<String> {
	// `then` keeps the quiet path free: the closure (and therefore the
	// allocation) never runs when the flag is off for this session.
	enabled_for(session).then(|| {
		let status = r.get("status").and_then(|v| v.as_str()).unwrap_or("?");
		let compressed = r.get("compressed").and_then(|v| v.as_bool()).unwrap_or(false);
		let ccr_type = r.get("type").and_then(|v| v.as_str()).unwrap_or("?");
		let size = r.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
		let hash = r.get("hash").and_then(|v| v.as_str()).unwrap_or("");
		let reason = r.get("reason").and_then(|v| v.as_str()).unwrap_or("");

		format!(
			"[aphrodite-debug: session={} status={} compressed={} type={} size={} hash={} reason={}]",
			session, status, compressed, ccr_type, size, hash, reason
		)
	})
}
