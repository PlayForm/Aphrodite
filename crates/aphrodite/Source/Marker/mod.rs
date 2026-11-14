//! CCR marker generation - 1:1 port of plugins/aphrodite/_marker/marker.py
//!
//! Generates <<<CCR:hash|type|size>>> markers with TOML-driven templates.
//!
//! 1.5.0: atomized into a per-item tree: `parse.rs` (hash/size parsing and
//! extraction), `format.rs` (ccr_marker rendering), `tests.rs` (battery).
//! This facade re-exports the exact public names the rest of the crate
//! (and `aphrodite-hermes` via `lib.rs`) depends on.

mod Format;
mod Parse;

#[cfg(test)]
mod Test;

pub use Format::{ccr_marker, set_marker_templates};
pub use Parse::{extract_hashes, is_valid_ccr_hash, normalize_hash, parse_preview};
pub(crate) use Format::substitute_vars;
#[cfg(test)]
pub(crate) use Format::{marker_hint_enabled, marker_templates_test_guard, registered_marker_format};
