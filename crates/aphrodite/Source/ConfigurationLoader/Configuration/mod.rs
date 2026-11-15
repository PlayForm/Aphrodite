//! The configuration model - one parsed `aphrodite.toml` plus the runtime
//! overrides that outrank it.
//!
//! `Struct` is inert: it holds the table and answers questions about it. It
//! does not know how the file was found (`Load`), how a single knob is
//! resolved (`Resolve`), or how a section reaches runtime state (`Apply`).
//! Keeping those three verbs in separate subtrees is what stops a resolution
//! chain from growing a second, subtly different copy in an applier.

pub mod Apply;
pub mod Default;
pub mod Load;
pub mod Resolve;
pub mod Struct;
