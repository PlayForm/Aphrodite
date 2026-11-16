//! The inline store's entry-count bound.
//!
//! Separate from the byte budget (`super::Const`) because the two bounds are
//! enforced by one eviction pass but tuned for different reasons: the count is
//! the historical cap, the bytes exist because the count alone never bounded
//! memory.

pub mod Const;
