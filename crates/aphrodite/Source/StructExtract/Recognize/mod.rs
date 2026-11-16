//! One recogniser per declaration shape.
//!
//! A recogniser answers one question - "is this line a declaration of my shape, and if so what are its
//! parts" - and returns a `Declaration`. It owns no budget, no truncation and no category key, which is
//! what keeps the four languages from drifting apart on any of the three.

pub mod Go;
pub mod JS;
pub mod Python;
pub mod Rust;
pub mod Type;
