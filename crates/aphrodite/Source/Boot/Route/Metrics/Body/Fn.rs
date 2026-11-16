//! The exposition body: the table, rendered in order.

use serde_json::Value;

use crate::Boot::Route::Metrics::{Piece, Table};

/// Render the stats document as Prometheus text exposition.
///
/// Kept as a pure function of the stats JSON so the exposition format can be
/// checked without an HTTP request. Two label choices are load-bearing and were
/// both wrong before: the last latency bucket is labelled `+Inf` because it has no
/// upper bound (`RecordLatency`'s final arm catches every sample >= 1s, so
/// calling it `10.0` made `histogram_quantile()` compute nonsense), and the
/// explicit `+Inf` bucket is what makes `_count` agree with the bucket sum.
///
/// Every section is an absent-or-present optional member of the stats document,
/// so a missing member emits no line at all. Nothing here branches on a value:
/// the order of `Table::ORDER` is the only thing that decides which metrics
/// exist, and `Piece::Render` answers one piece with its lines.
pub fn Fn(Stats:&Value) -> String {
	Table::Const::ORDER.iter().map(|Piece| Piece::Render::Fn::Fn(*Piece, Stats)).collect()
}
