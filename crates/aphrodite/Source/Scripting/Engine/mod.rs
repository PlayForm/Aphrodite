//! The scripting engine.
//!
//! `Struct/` is the type and its constructor, then one leaf per concern:
//! `Reload/` rescans the script dirs, `Check/` is the mtime trigger, `Run/`
//! is the hook call itself, and `Compress/`, `Marker/`, `Retrieve/` are the
//! three pre-Rust compatibility surfaces over it. Each leaf is named for the
//! method it defines, so `Engine::Compress::Fn` is the `Compress` surface and
//! there is one way to spell it.
//!
//! The `.rhai` hook function each surface drives is a separate, script-facing
//! contract - `on_compress`, `on_marker`, `on_retrieve` - and is passed as a
//! string at the call site. Renaming a method here does not rename a hook.
//!
//! Every leaf is an `impl` block on the same type. They are split by concern
//! rather than by visibility so the 159-line impl never hides a whole behaviour
//! behind a scroll - the reload path and the call path are different failure
//! modes and are read for different reasons.

pub mod Check;
pub mod Compress;
pub mod Marker;
pub mod Reload;
pub mod Retrieve;
pub mod Run;
pub mod Struct;
