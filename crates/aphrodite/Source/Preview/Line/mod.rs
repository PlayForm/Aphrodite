//! Per-line predicates shared by the preview detectors and builders.
//!
//! A preview arm and the detector that routes to it must agree about what a
//! line is; if they do not, content is classified as one shape and rendered as
//! another. Each predicate therefore lives here exactly once and both sides
//! call it. The families are grouped by what they read - code, errors,
//! failures, git status, grep hits, lint issues, markdown, paths, test output,
//! warnings, YAML keys - and a family holding more than one predicate is a
//! directory of one-predicate leaves.

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
