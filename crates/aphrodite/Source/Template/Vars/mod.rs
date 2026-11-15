//! The template variable set.
//!
//! `Fn/` is the whole set, assembled in four passes: `Default/` (the flat defaults
//! every name starts at), `Derive/` (the content-derived values, and the only place a
//! "not found" placeholder is spelled), `Code/` (the structure map for code content,
//! as a typed record) and `Computed/` (the variables derived from other variables).
//! `Stat/` is the line statistics `Derive/` consumes.
//!
//! The split is per variable family because the pre-Rust variable set is a
//! compatibility contract: a template written against the Python engine names any of
//! these, so each family must be findable on its own when one of them drifts. Every
//! value degrades to its pre-Rust default - a template variable that cannot be
//! derived is `"?"`/`""`/`"0"`, never a panic.

pub mod Code;
pub mod Computed;
pub mod Default;
pub mod Derive;
pub mod Fn;
pub mod Stat;
