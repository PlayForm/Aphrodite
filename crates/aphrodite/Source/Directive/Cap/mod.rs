//! The directive size caps.
//!
//! `Fn/` cuts one directive body to the per-file cap and the two `Maximum*` leaves hold
//! the numbers. Both caps are characters, and both are applied by one function each, so a
//! cap can only be wrong in one place.

pub mod Fn;
pub mod MaximumCombinedChars;
pub mod MaximumDirectiveChars;
