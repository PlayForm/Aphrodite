//! The value a parser produces and a renderer consumes.
//!
//! A summary is a domain aggregate, not a formatted string: it owns the tallies, the
//! bounded collections and the ordering rules, and it exposes them through accessors
//! so the renderer cannot re-derive them differently. Truncation for display lives in
//! `Render`, which is why a summary holds whole subjects and whole paths.

pub mod Git;
pub mod GitLog;
pub mod Listing;
pub mod Markdown;
pub mod Search;
