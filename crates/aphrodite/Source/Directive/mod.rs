//! Conversational Directives - lightweight behavioral context for the LLM.
//!
//! Directives are short `.md` files that inject behavioral instructions into
//! the LLM's context via `PreLLMCall`. Unlike file content (which gets
//! compressed into CCR markers), directives are **always inline** - they're
//! compact enough to never need compression, and the engine injects them
//! directly into the LLM's context without any file-system round-trip or
//! retrieval step.
//!
//! Per profile, per user: `directives/*.md` files are swappable at runtime
//! via `aphrodite_directive("swap", "name")`.
//!
//! Atomized into a reverse-taxonomy tree: `Builtin.rs` (baked-in include_str!
//! defaults), `Model.rs` (the directive struct + size caps), `Load.rs` (disk
//! loading), `Build.rs` (context injection), `Action.rs` (runtime actions),
//! `Test/` (the battery, split by area). Each leaf owns exactly one item and
//! names it by kind, so the file says what the item is and the directory says
//! what it is for.
//!
//! This facade declares the tree and nothing else. A consumer names the leaf
//! (`Directive::Model::Struct::Struct`), so the file that defines an item is
//! the only place it is defined and there is exactly one way to spell it.

pub mod Action;
pub mod Build;
pub mod Builtin;
pub mod Load;
pub mod Model;

#[cfg(test)]
mod Test;
