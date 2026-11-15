//! Test-output line predicates.
//!
//! Cargo, pytest and jest all describe a run as a header, a stream of result
//! lines, and a summary count. The test arm reports all three, so each shape
//! gets its own predicate here rather than one overloaded matcher.

pub mod NumberAfter;
pub mod NumberBefore;
pub mod Result;
pub mod Running;
