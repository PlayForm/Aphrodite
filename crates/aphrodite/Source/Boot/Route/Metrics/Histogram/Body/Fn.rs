//! The histogram lines, once the document is known to carry buckets.

use serde_json::Value;

use crate::Boot::Route::Metrics::Histogram::Bucket;

/// Fold the buckets into cumulative lines, then close with the count and, when
/// the document holds one, the total latency.
///
/// The buckets are cumulative by definition: a Prometheus bucket is `<= le`, not
/// `== le`, so each line carries the running total rather than its own count.
pub fn Fn(Stats:&Value, Buckets:&[Value]) -> String {
	let Filled = Buckets.iter().enumerate().fold(Bucket::New::Fn::Fn(), Bucket::Step::Fn::Fn);
	let Count = format!("aphrodite_latency_seconds_count {}\n", Filled.Total);
	let Sum = Stats["total_latency_micros"]
		.as_u64()
		.map(|TotalUs| format!("aphrodite_latency_seconds_sum {:.6}\n", TotalUs as f64 / 1_000_000.0))
		.unwrap_or_default();
	let mut Out = Filled.Lines;
	Out.push_str(&Count);
	Out.push_str(&Sum);
	Out
}
