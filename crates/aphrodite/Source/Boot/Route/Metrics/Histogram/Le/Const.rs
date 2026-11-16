//! The bucket boundaries.

/// The bucket boundaries, in order.
///
/// The last one is `+Inf` because `RecordLatency`'s final arm has no upper bound
/// (it catches every sample >= 1s, including 30s+ outliers), so labelling it
/// `10.0` was actively wrong: Prometheus consumers would assume everything in it
/// is <= 10s and mis-compute quantiles. The explicit `+Inf` bucket is also what
/// makes `_count` agree with the bucket sum, which `histogram_quantile()`
/// requires.
pub const LABELS:[&str; 5] = ["0.001", "0.01", "0.1", "1.0", "+Inf"];
