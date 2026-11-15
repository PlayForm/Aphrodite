//! The line scan both query filters share, and the case fold it applies.
//!
//! `Match/` returns the lines that contain a lowercased query; `Fold/` is which lowercasing
//! it compares against. They live outside `Filter.rs` because that file's public path is its
//! one item (`Resolve::Filter::Fn`), and these are two more items - so they get their own
//! domain name rather than a directory level inserted above the contract.

pub mod Fold;
pub mod Match;
