//! `aphrodite_catalog` - full or table-of-contents view of recorded markers.

use crate::Tool::{Arg, Dispatch::ToolHandler};

/// The `aphrodite_catalog` handler.
///
/// Newest first, unfiltered: this is the reliable fallback when a search misses.
/// `toc` mode drops `turn`/`center` because its whole purpose is to be cheap to
/// read; `full` is the default so a caller that passes no mode gets everything.
pub(crate) fn Fn() -> (&'static str, ToolHandler) {
	("aphrodite_catalog", |args| {
		let mode = {
			let m = Arg::Fn(args, "mode");
			if m.is_empty() { "full" } else { m }
		};
		crate::with_shared(|state| {
			let items:Vec<serde_json::Value> = state
				.recent_markers
				.iter()
				.rev()
				.map(|e| {
					if mode == "toc" {
						serde_json::json!({
							"hash": &e.hash,
							"type": e.ccr_type, "size": e.size, "preview": e.preview,
						})
					} else {
						serde_json::json!({
							"hash": e.hash, "type": e.ccr_type, "size": e.size,
							"preview": e.preview, "turn": e.turn, "center": e.center,
						})
					}
				})
				.collect();
			serde_json::json!({"mode": mode, "total": items.len(), "items": items, "turn": state.turn_counter})
		})
	})
}
