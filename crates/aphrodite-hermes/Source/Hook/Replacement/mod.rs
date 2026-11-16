//! Map a hook compression result to the value Hermes uses to replace output.
//!
//! The invisibility contract in code: the model must see the same kind of thing
//! for every compressed output - a natural CCR marker (plus its preview) that it
//! can retrieve by hash - and never the mechanism's own vocabulary. Returning
//! `null` is a pass-through (leave the original output untouched); returning a
//! string REPLACES it, so a wrong branch here either erases output or leaks
//! internals into the transcript.
//!
//! `Choice/` holds that contract as a value: one variant per outcome, one leaf per
//! outcome to detect it, `Resolve/` for the lazy precedence chain, and this leaf
//! for the render.

pub mod Choice;
pub mod Fn;
