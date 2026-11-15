//! The payload for a hook that deliberately transformed nothing.

/// The `compressed: false` payload, from the reason that produced it.
///
/// One shape for every passthrough: the field order and the `status`/`compressed`
/// pair are what the bridge and the plugin's tests read, so a per-gate payload
/// would let one of them drift into a different JSON object with the same
/// meaning.
pub(crate) fn Fn(Reason:&str) -> serde_json::Value {
	serde_json::json!({ "status": "ok", "compressed": false, "reason": Reason })
}
