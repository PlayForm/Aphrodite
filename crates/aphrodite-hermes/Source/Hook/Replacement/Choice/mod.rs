//! What Hermes substitutes for a compressed result.
//!
//! One variant per outcome, each carrying what it needs and nothing else. The
//! variants are the lifecycle: `Passthrough` is the absence of a replacement,
//! and it is NOT an empty string - Hermes honours any string return as the
//! replacement, so an empty string erases the output instead of leaving it
//! alone.

pub mod Chain;
pub mod Enum;
pub mod Expanded;
pub mod Marker;
pub mod Resolve;
