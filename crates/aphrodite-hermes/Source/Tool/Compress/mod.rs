//! Store-and-marker: the shared step behind `aphrodite_compress`, the smoke test
//! and every hook that turns content into a resolvable CCR entry.
//!
//! The one rule this module exists to enforce: the ORIGINAL content is hashed and
//! stored, and the unwrapped fragment is used only to choose a type and preview.
//! Retrieval must return exactly what was passed in - wrapper metadata like
//! `exit_code` included - or the tool is lossy in the one way that matters.
//!
//! `Classified/` holds that choice as a value: `Resolve/` decides which body and
//! which type the preview uses, and `Fn/` hashes and stores the original.

pub mod Classified;
pub mod Fn;
