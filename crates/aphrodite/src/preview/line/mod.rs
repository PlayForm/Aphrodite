//! Per-line predicate primitives, one file per predicate family
//! (1.5.0 reverse-taxonomy split of the old monolithic `preview.rs`).
//!
//! Shared by detectors (shape votes) and builders (preview arms) - each
//! predicate lives here exactly once.

pub mod code;
pub mod error;
pub mod failure;
pub mod git_status;
pub mod grep;
pub mod lint;
pub mod md_heading;
pub mod md_structure;
pub mod path;
pub mod test;
pub mod warning;
pub mod yaml_key;
