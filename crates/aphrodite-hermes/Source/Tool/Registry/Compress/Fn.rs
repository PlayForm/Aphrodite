//! `aphrodite_compress` - store content, return a resolvable marker.

use crate::Tool::{Arg, Compress, Dispatch::ToolHandler};

/// The `aphrodite_compress` handler.
///
/// An empty `content` is rejected rather than stored: a zero-byte entry would
/// hash to the same value as every other empty payload and shadow it in the
/// inline store.
pub(crate) fn Fn() -> (&'static str, ToolHandler) {
	("aphrodite_compress", |args| {
		let content = Arg::Fn(args, "content");
		if content.is_empty() {
			return serde_json::json!({"error": "content is required"});
		}
		let hint = Arg::Fn(args, "type");
		let center = args.get("_ccr_center").and_then(|v| v.as_str());
		crate::with_shared(|state| Compress::Fn(state, content, hint, center))
	})
}
