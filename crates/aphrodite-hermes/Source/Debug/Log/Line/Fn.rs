//! The `[aphrodite-debug ...]` line prepended to a compressed tool result.
//!
//! This is the only debug surface the MODEL sees: the flag is on, so instead of
//! retrieving the marker the session can read what the engine did inline. It is
//! therefore deliberately a single line of scalar fields - never the payload,
//! never a multi-line block that would compete with the marker.

use crate::Debug::Flag;

/// Build a `[aphrodite-debug ...]` prefix line from a compression result
/// when the flag is on for `session`; `None` when off (no allocation on the
/// quiet path).
///
/// It is not a log sink - the line is returned, not written, because its
/// destination is the tool-result stream. Missing fields render as `?`/`0`
/// rather than being omitted, so the line has a stable shape to read: the
/// `key=value` names in the emitted line (`status`, `compressed`, `type`,
/// `size`, `hash`, `reason`) are the wire form and do not follow a rename.
pub(crate) fn Fn(Record:&serde_json::Value, Session:&str) -> Option<String> {
	// `then` keeps the quiet path free: the closure (and therefore the
	// allocation) never runs when the flag is off for this session.
	Flag::Resolve::Fn::Fn(Session).then(|| {
		let Status = Record.get("status").and_then(|Field| Field.as_str()).unwrap_or("?");
		let Compressed = Record.get("compressed").and_then(|Field| Field.as_bool()).unwrap_or(false);
		let Type = Record.get("type").and_then(|Field| Field.as_str()).unwrap_or("?");
		let Size = Record.get("size").and_then(|Field| Field.as_u64()).unwrap_or(0);
		let Hash = Record.get("hash").and_then(|Field| Field.as_str()).unwrap_or("");
		let Reason = Record.get("reason").and_then(|Field| Field.as_str()).unwrap_or("");

		format!(
			"[aphrodite-debug: session={} status={} compressed={} type={} size={} hash={} reason={}]",
			Session, Status, Compressed, Type, Size, Hash, Reason
		)
	})
}
