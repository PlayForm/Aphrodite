//! `path:line:` result parsing.
//!
//! Two producers read the same shape with genuinely different acceptance rules: a grep
//! hit needs a file-ish path, a search result does not. They share the parsed record and
//! the summary, and keep their own parser - collapsing them into one predicate would
//! silently move lines from one arm to the other.

pub mod Grep;
pub mod Hit;
pub mod Plain;
