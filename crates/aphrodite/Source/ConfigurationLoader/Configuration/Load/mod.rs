//! Producing a `Struct`: where the file is searched for, how an explicit path
//! is read, how a reload replaces one, and how a runtime override is set.
//!
//! One file per concern. Every path through this subtree degrades to defaults
//! on failure - a broken `aphrodite.toml` must never stop the engine from
//! starting, only make it louder.

pub mod File;
pub mod Override;
pub mod Reload;
pub mod Search;
