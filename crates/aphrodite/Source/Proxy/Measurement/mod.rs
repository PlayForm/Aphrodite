//! Accounting, in two units that must not be added together.
//!
//! `Content` measures the stored content a rewrite replaced - the model's context
//! shrinks by that much. `Encoded` measures the serialised HTTP body a cache hit
//! avoided - bandwidth. A hit and a rewrite both "save bytes" in ordinary speech
//! and in neither of these units, which is why they are separate values feeding
//! separate counters rather than one number that describes neither.

pub mod Content;
pub mod Encoded;
pub mod Estimate;
