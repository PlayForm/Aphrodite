//! The template variable set.
//!
//! `Fn/` is the whole set, assembled from four families: `Base/` (the flat
//! defaults plus the content-derived values), `Stat/` (the line statistics),
//! `Code/` (the structure map for code content) and `Computed/` (the variables
//! derived from other variables).
//!
//! The split is per variable family because the pre-Rust variable set is a
//! compatibility contract: a template written against the Python engine names
//! any of these, so each family must be findable on its own when one of them
//! drifts. Every value degrades to its pre-Rust default - a template variable
//! that cannot be derived is `"?"`/`""`/`"0"`, never a panic.

pub mod Base;
pub mod Code;
pub mod Computed;
pub mod Fn;
pub mod Stat;
