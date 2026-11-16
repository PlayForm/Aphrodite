//! The budget's percentage to its truncation.

use super::Enum::Enum as Limit;

/// Turn a headroom budget into the truncation it asks for.
///
/// The thresholds are the pre-Rust Python port's, kept byte-for-byte: a budget
/// under 25 keeps 30 characters, under 50 keeps 60, under 75 keeps 100, and 75
/// or more keeps the whole preview. The table lives here so the entry point asks
/// one question instead of holding a range match of its own.
///
/// `None` (no budget configured) and a budget at or above 75 are the same
/// outcome, and that is deliberate rather than an accident of the last arm: both
/// mean the preview is not truncated at all.
pub(crate) fn Fn(HeadroomBudget:Option<u32>) -> Limit {
	match HeadroomBudget {
		None | Some(75..) => Limit::Unlimited,
		Some(0..=24) => Limit::At(30),
		Some(25..=49) => Limit::At(60),
		Some(50..=74) => Limit::At(100),
	}
}
