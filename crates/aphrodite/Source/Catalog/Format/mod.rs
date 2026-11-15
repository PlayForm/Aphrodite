//! The markdown catalog table.
//!
//! `Fn/` is the two-case decision (empty catalog versus a table) and `Table/` is the
//! renderer for the non-empty one. The split is what keeps the "No compressed items
//! yet." body from living inside a builder that has already emitted a header.

pub mod Fn;
pub mod Table;
