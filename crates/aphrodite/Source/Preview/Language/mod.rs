//! Per-language code refinement.
//!
//! Consulted only when the shape detector already said `"code"`, and only to name the
//! language. Each language's evidence rule is its own leaf, because the four rules are four
//! different acceptance criteria and the order they are tried in is the precedence.

pub mod Fn;
pub mod Go;
pub mod JS;
pub mod Python;
pub mod Rust;
