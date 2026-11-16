//! `aphrodite_catalog` - full or table-of-contents view of recorded markers.

use crate::Tool::{Arg, Dispatch::Fn::Handler};

/// The `aphrodite_catalog` handler.
///
/// Newest first, unfiltered: this is the reliable fallback when a search misses.
/// `toc` mode drops `turn`/`center` because its whole purpose is to be cheap to
/// read; `full` is the default so a caller that passes no mode gets everything.
pub(crate) fn Fn() -> (&'static str, Handler) {
	("aphrodite_catalog", |Args| {
		let Mode = match Arg::Fn::Fn(Args, "mode") {
			"" => "full",
			Mode => Mode,
		};

		crate::Session::With::Fn::with_shared(|State| {
			let Items:Vec<serde_json::Value> = State
				.Marker
				.Recent
				.iter()
				.rev()
				.map(|Marker| {
					match Mode {
						"toc" => {
							serde_json::json!({
								"hash": &Marker.hash,
								"type": Marker.CCRType, "size": Marker.Size, "preview": Marker.Preview,
							})
						},
						_ => {
							serde_json::json!({
								"hash": Marker.hash, "type": Marker.CCRType, "size": Marker.Size,
								"preview": Marker.Preview, "turn": Marker.Turn, "center": Marker.Center,
							})
						},
					}
				})
				.collect();

			serde_json::json!({"mode": Mode, "total": Items.len(), "items": Items, "turn": State.Turn.Counter})
		})
	})
}
