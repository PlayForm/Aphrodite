//! Test battery for the resolver, split by area.
//!
//! These pin the resolution contract: what a marker parses to, what a filter keeps
//! and drops, what a single lookup returns, what an expansion produces, and what
//! the recursive walk does at a cycle or a depth limit. They are the reason the
//! split into one-item-per-file was allowed at all - a green run before and after
//! is the proof that nothing but the layout changed. No test here may be weakened
//! to fit a refactor: each one names a bug that already happened once.
//!
//! The shared imports live here so every area file reads `use super::*;`.

use std::collections::HashMap;

mod Expand;
mod Filter;
mod Marker;
mod One;
mod Properties;
mod Recursive;
