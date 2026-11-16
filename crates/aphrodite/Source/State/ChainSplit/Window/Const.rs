//! The adaptation window's minimum.

/// Split events that must be recorded before the threshold adapts at all.
///
/// The floor is about noise, not memory: one split's retrieval ratio is not a
/// trend, so below this the adaptation holds instead of moving the threshold.
pub(crate) const Minimum:usize = 4;
