//! The scripting engine.
//!
//! `Struct/` is the type and its constructor, then one leaf per concern:
//! `Reload/` rescans the script dirs, `CheckReload/` is the mtime trigger,
//! `Run/` is the hook call itself, and `OnCompress/`, `OnMarker/`,
//! `OnRetrieve/` are the three pre-Rust compatibility surfaces over it.
//!
//! Every leaf is an `impl` block on the same type. They are split by concern
//! rather than by visibility so the 159-line impl never hides a whole behaviour
//! behind a scroll - the reload path and the call path are different failure
//! modes and are read for different reasons.

pub mod CheckReload;
pub mod OnCompress;
pub mod OnMarker;
pub mod OnRetrieve;
pub mod Reload;
pub mod Run;
pub mod Struct;
