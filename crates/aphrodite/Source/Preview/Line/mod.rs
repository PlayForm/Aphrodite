//! Per-line predicate primitives, one file per predicate family
//! (1.5.0 reverse-taxonomy split of the old monolithic `preview.rs`).
//!
//! Shared by detectors (shape votes) and builders (preview arms) - each
//! predicate lives here exactly once.

pub mod Code;
pub mod Error;
pub mod Failure;
pub mod GitStatus;
pub mod Grep;
pub mod Lint;
pub mod MarkdownHeading;
pub mod MarkdownStructure;
pub mod Path;
pub mod Test;
pub mod Warning;
pub mod YAMLKey;
