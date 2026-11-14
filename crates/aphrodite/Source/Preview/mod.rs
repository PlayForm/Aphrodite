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

pub mod Builder;
pub mod Detect;
pub mod Detector;
pub mod Input;
#[cfg(feature = "proxy")]
pub mod Language;
pub mod Line;
pub mod Type;
pub mod State;
pub mod Text;

#[cfg(test)]
mod Test;

pub use Builder::build_preview;
pub use Detect::{detect_semantic_type, detect_type};
pub use State::{preview_max_chars, set_preview_max_chars};
#[cfg(test)]
pub(crate) use State::preview_cap_test_guard;

// TOML-driven preview template rendering (pre-Rust port): re-exported here
// so engine/proxy/hermes call sites keep one `aphrodite::Preview::` prefix
// for every preview knob (the `[templates.preview]` registry is consumed by
// `build_preview` above).
pub use crate::Template::{get_preview_family, get_templates, render_preview, set_preview_family, set_templates};
