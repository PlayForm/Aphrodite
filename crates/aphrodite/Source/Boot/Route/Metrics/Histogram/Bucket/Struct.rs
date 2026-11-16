//! The histogram fold's accumulator.

/// The running total and the lines written so far.
pub struct Struct {
	/// Cumulative sample count across the buckets folded so far.
	pub Total:u64,
	/// The bucket lines written so far, in order.
	pub Lines:String,
}
