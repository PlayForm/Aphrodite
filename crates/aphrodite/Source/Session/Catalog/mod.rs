//! The recall-catalog summary.
//!
//! One leaf per section of the line, in emission order, and `Fn.rs` joins them:
//! `MarkerDelta/`, `Archived/`, `FileDelta/`, `Workers/`, `Intent/`. A section
//! that has nothing to report answers `None`, so the join never leaves a stray
//! separator and the model never reads a section that says nothing.

pub mod Archived;
pub mod FileDelta;
pub mod Fn;
pub mod Intent;
pub mod MarkerDelta;
pub mod Workers;
