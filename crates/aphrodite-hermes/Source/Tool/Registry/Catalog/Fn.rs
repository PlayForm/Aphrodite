//! `aphrodite_catalog` - full or table-of-contents view of recorded markers.

use crate::Tool::{Arg, Dispatch::Fn::ToolHandler};

/// The `aphrodite_catalog` handler.
///
/// Newest first, unfiltered: this is the reliable fallback when a search misses.
/// `toc` mode drops `turn`/`center` because its whole purpose is to be cheap to
/// read; `full` is the default so a caller that passes no mode gets everything.
pub(crate) fn Fn() -> (&'static str, ToolHandler) {
	("aphrodite_catalog", |args| {
		let mode = {
			let m = Arg::Fn::Fn(args, "mode");
			if m.is_empty() { "full" } else { m }
		};
		crate::Session::With::Fn::with_shared(|state| {
			let items:Vec<serde_json::Value> = state
				.RecentMarkers
				.iter()
				.rev()
				.map(|e| {
					if mode == "toc" {
						serde_json::json!({
							"hash": &e.hash,
							"type": e.CCRType, "size": e.Size, "preview": e.Preview,
						})
					} else {
						serde_json::json!({
							"hash": e.hash, "type": e.CCRType, "size": e.Size,
							"preview": e.Preview, "turn": e.Turn, "center": e.Center,
						})
					}
				})
				.collect();
			serde_json::json!({"mode": mode, "total": items.len(), "items": items, "turn": state.TurnCounter})
		})
	})
}
