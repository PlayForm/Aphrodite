//! Conversational Directives - lightweight behavioral context for the LLM.
//!
//! Directives are short `.md` files that inject behavioral instructions into the LLM's context
//! via `PreLLMCall`. Unlike file content (which gets compressed into CCR markers), directives are
//! **always inline** - they are compact enough to never need compression, and the engine injects
//! them directly into the LLM's context without any file-system round-trip or retrieval step.
//!
//! Per profile, per user: `directives/*.md` files are swappable at runtime via
//! `aphrodite_directive("swap", "name")`.
//!
//! The tree follows the data's own path: `Model.rs` is the directive type, `Embedded/` is the
//! baked-in `include_str!` table, `Builtin.rs` loads it, `Entry/` reads one directory entry,
//! `Load.rs` walks the directory, `Cap/` holds the two size caps and the one function that
//! applies the per-file one, `Build.rs` renders the injected context and `Action.rs` dispatches
//! the runtime verbs (`Verb/`). `Refusal/` is the one place both of the vocabulary's refusals are
//! spelled, and `Test/` is the battery, split by area.
//!
//! Directive identifiers are DATA, not names: a directive is addressed by the lowercase stem of
//! its `.md` file (`focus`), which is also its map key and the string `aphrodite_directive`
//! accepts. Nothing here renames one.
//!
//! A module that owns exactly one item stays a FILE (`Model.rs`, `Build.rs`, `Load.rs`,
//! `Builtin.rs`, `Action.rs`), because its public path is the item's path: turning it into a
//! directory would insert a segment and break every consumer. Only a module that genuinely holds
//! several leaves becomes a directory.
//!
//! This facade declares the tree and nothing else. A consumer names the leaf
//! (`Directive::Model::Struct`), so the file that defines an item is the only place it is defined
//! and there is exactly one way to spell it.

pub mod Action;
pub mod Build;
pub mod Builtin;
pub mod Cap;
pub mod Embedded;
pub mod Entry;
pub mod Load;
pub mod Model;
pub mod Refusal;
pub mod Verb;

#[cfg(test)]
mod Test;
