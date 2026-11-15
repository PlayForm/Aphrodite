//! Test battery for the directives module, split by area.
//!
//! `Build.rs` pins the injected context string (full body, char-based cap),
//! `Action.rs` the runtime action vocabulary and the ephemeral/latch rules,
//! `Load.rs` the disk loader's Issue #6 semantics and the built-in set.

mod Action;
mod Build;
mod Load;
