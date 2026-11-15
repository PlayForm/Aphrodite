//! Per-shape line parsers - the first stage of every rewritten builder.
//!
//! A parser reads the text a tool printed and produces a domain record per line (or,
//! for the two shapes whose aggregation is inseparable from their recognition, the
//! summary value directly). It never truncates for display, never lays out a
//! collection and never names a wire category: those are the renderer's job, which is
//! what lets the acceptance rule and the output shape change independently.

pub mod Git;
pub mod GitLog;
pub mod Listing;
pub mod Markdown;
pub mod Search;
