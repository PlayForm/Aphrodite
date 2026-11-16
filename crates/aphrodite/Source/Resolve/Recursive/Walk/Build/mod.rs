//! The expansion itself, split from the path that owns it.
//!
//! `Fn/` loads the entry, resolves each occurrence, and builds the output against the
//! traversal's budget. It is a separate leaf from `Walk::Fn` because the two have different
//! obligations: `Walk::Fn` owns the path - enter before, leave after, whatever the outcome -
//! while `Fn` owns the document and never touches the path itself.

pub mod Fn;
