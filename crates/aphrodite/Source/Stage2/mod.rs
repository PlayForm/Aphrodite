//! Stage 2 compression - semantic reduction of CCR-stored content.
//! Port of plugins/aphrodite/_stage2.py
//!
//! Produces a denser version of a piece of content on demand via the
//! stateless `aphrodite_stage2(content, ccr_type)` ABI (see
//! `Library.rs::aphrodite_stage2`) - it takes content directly and returns the
//! reduced string, with no interaction with the CCR store or the session's
//! inline store.
//!
//! There is currently no depth-aware retrieval: no `AphroditeRetrieve` /
//! `ResolveOne` / `Resolve::Expand::Fn` entry point accepts a `Depth` parameter,
//! and nothing in this crate writes a stage-2-reduced value back into the
//! store under a `{hash}#stage2` key for a later plain retrieval to pick up
//! (report 05 F6 - a version of this docstring used to claim
//! `aphrodite_retrieve(hash, depth=2)` returns the reduced version; that
//! entry point does not exist). Wiring depth-aware retrieval up for real is
//! a deliberate feature decision - see `.plans/05-compression-pipeline.md`
//! §5 - not something this module does today.
//!
//! Reducers per content type:
//! - JSON: whitespace minification + key extraction
//! - Build/log: error/warning/summary extraction
//! - Diff: file-level summary with hunk counts
//! - Code: function/struct/class signature extraction
//! - Other: pass through (return None = no reduction)
//!
//! The tree is one file per item, so a new content type is a new reducer plus
//! one registry line instead of an edit inside a monolith:
//!
//! - `Const.rs` - the size floor below which no reduction is attempted.
//! - `Reducer/Type.rs` - the `fn(&str) -> Option<String>` shape of a reducer.
//! - `Reduce/JSON|Build|Diff|Code/Fn.rs` - the four reducers.
//! - `Name/Fn.rs` - the JSON value-kind label the JSON header prints.
//! - `Regex/Fn.rs` - the prefix matcher the code reducer uses instead of a
//!   regex crate dependency.
//! - `Registry/Fn.rs` - CCR type string → reducer.
//! - `Compress/Fn.rs` - the public entry point.
//!
//! This facade declares the tree and re-exports the entry point under the name
//! the rest of the crate and `aphrodite-hermes` already call it by. Nothing
//! else belongs in this file.

pub mod Compress;
pub mod Const;
pub mod Name;
pub mod Reduce;
pub mod Reducer;
pub mod Regex;
pub mod Registry;

#[cfg(test)]
pub mod Test;

