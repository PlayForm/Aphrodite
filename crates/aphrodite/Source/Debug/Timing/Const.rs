//! The timing accumulators.

use std::sync::atomic::AtomicU64;

/// DEV-ONLY timing sums, in microseconds, per category.
///
/// Read together with the sample counts below - a sum without its count is not
/// an average. The two arrays are declared side by side so a new category
/// cannot be added to one and forgotten in the other.
pub(crate) static TimingSum:[AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];
/// DEV-ONLY timing sample counts, per category (the divisor for `TimingSum`).
pub(crate) static TimingN:[AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];
