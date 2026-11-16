//! The latency histogram block.

use serde_json::Value;

use crate::Boot::Route::Metrics::Histogram::Body;

/// The five bucket lines, the count and the sum - or nothing at all when the
/// stats document carries no latency buckets.
pub fn Fn(Stats:&Value) -> String {
	Stats["latency_buckets_us"].as_array().map_or_else(String::new, |Buckets| Body::Fn::Fn(Stats, Buckets))
}
