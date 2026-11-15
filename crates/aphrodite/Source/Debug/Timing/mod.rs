//! The timing accumulators.
//!
//! `Enum/` names the categories, `Const/` holds the sum and the sample count
//! per category, `Add/` is the only writer. Sum-and-count rather than a running
//! average: an average updated in place drifts and cannot be re-derived, while
//! a sum and a count give `MetricsJSON` the same answer at any point in the
//! session.

pub mod Add;
pub mod Const;
pub mod Enum;
