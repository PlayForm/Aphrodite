//! The timing accumulators.

use std::sync::atomic::AtomicU64;

use crate::Debug::Timing::Enum::Enum as TimingCat;

/// Timing categories - the length of both storage arrays.
///
/// Private to this file and checked against the enum below, so the sum array, the
/// count array and the discriminants cannot drift apart.
const Categories:usize = 4;

/// DEV-ONLY timing sums, in microseconds, per category.
///
/// Read together with the sample counts below - a sum without its count is not
/// an average. Both arrays are declared side by side, at the same length.
pub(crate) static TimingSum:[AtomicU64; Categories] = [const { AtomicU64::new(0) }; Categories];
/// DEV-ONLY timing sample counts, per category (the divisor for `TimingSum`).
pub(crate) static TimingN:[AtomicU64; Categories] = [const { AtomicU64::new(0) }; Categories];

/// Both arrays hold exactly one slot per enum discriminant.
///
/// A compile-time assertion rather than a comment: a category added to the enum
/// without growing `Categories` fails to compile here, instead of indexing past the
/// end of an accumulator at the first `fetch_add`.
const _:() = assert!(Categories == TimingCat::Render as usize + 1);
