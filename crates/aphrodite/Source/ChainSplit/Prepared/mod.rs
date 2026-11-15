//! A chain's artifacts, built before the session is touched.
//!
//! `Prepare/` is pure and `Apply/` is the only mutating leaf, so the split
//! decision and the store decision stay separable - and a caller that only
//! wants to know whether content is a chain never has to take the state lock.

pub mod Apply;
pub mod Prepare;
pub mod Segment;
pub mod Struct;
