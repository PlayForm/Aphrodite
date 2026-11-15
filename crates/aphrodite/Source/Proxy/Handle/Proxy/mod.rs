//! The catch-all proxy handler and its three stages.
//!
//! `Fn` is the entry point; `Lookup`, `Forward` and `Respond` are the stages it
//! delegates to, one failure mode each.

pub mod Fn;
pub mod Lookup;
pub mod Respond;
pub mod Forward;
