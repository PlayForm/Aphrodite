//! Fold one bucket into the accumulator.

use serde_json::Value;

use crate::Boot::Route::Metrics::Histogram::{Bucket, Le};

/// Add one bucket: its samples to the running total, its line to the output.
///
/// A bucket past the last named boundary is `+Inf`, which is why the boundary
/// lookup falls back rather than skipping the bucket - the final arm of
/// `RecordLatency` catches every remaining sample, so there is always a bucket
/// with no upper bound.
pub fn Fn(Bucket:Bucket::Struct::Struct, (Index, Value):(usize, &Value)) -> Bucket::Struct::Struct {
	let Boundary = Le::Const::LABELS.get(Index).copied().unwrap_or("+Inf");
	let Total = Bucket.Total + Value.as_u64().unwrap_or(0);
	let mut Lines = Bucket.Lines;
	Lines.push_str(&format!("aphrodite_latency_seconds_bucket{{le=\"{Boundary}\"}} {Total}\n"));
	Bucket::Struct::Struct { Total, Lines }
}
