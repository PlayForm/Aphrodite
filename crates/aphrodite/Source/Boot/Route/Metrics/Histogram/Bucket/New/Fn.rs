//! An empty accumulator.

use crate::Boot::Route::Metrics::Histogram::Bucket;

/// No buckets folded yet.
pub fn Fn() -> Bucket::Struct::Struct {
	Bucket::Struct::Struct { Total:0, Lines:String::new() }
}
