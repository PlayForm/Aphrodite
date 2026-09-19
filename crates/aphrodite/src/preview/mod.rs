//! Preview builder: content-type detection and human-readable preview strings
//! for compressed CCR content.
//!
//! Used by both the proxy binary and the `aphrodite-hermes` bridge crate
//! (via `crate::preview` re-export).
//!
//! 1.5.0: atomized into a reverse-taxonomy tree (REFACTOR-PLAN-1.5.0):
//! `detect.rs` (entry points), `builders/` (one file per preview arm),
//! `detectors/` (one file per shape), `line/` (per-line predicates),
//! `text/` (&str utilities), `state.rs` (preview cap), `tests.rs` (battery).
//! This facade re-exports the exact public names the rest of the crate
//! (and `aphrodite-hermes` via `lib.rs`) depends on.

pub mod builders;
pub mod detect;
pub mod detectors;
pub mod input;
pub mod lang;
pub mod line;
pub mod r#type;
pub mod state;
pub mod text;

#[cfg(test)]
mod tests;

pub use builders::build_preview;
pub use detect::{detect_semantic_type, detect_type};
pub use state::{preview_max_chars, set_preview_max_chars};

#[cfg(test)]
pub(crate) use state::preview_cap_test_guard;