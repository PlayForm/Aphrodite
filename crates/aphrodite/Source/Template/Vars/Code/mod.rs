//! The code-structure enrichment.
//!
//! `Struct/` is the typed record the extractor produces and `Fn/` is the reader that
//! produces it. Splitting them is what lets the renderer apply the namespace's
//! per-field defaults once, instead of the reader formatting five strings.

pub mod Fn;
pub mod Struct;
