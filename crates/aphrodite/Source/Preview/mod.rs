//! Preview: content-type detection and the honest preview string for compressed
//! CCR content.
//!
//! Used by both the proxy binary and the `aphrodite-hermes` bridge crate. The tree
//! is a reverse taxonomy: `Builder/` holds one arm per content type, `Detector/` one
//! shape vote per type, `Line/` the per-line predicates both sides share, `Text/` the
//! `&str` utilities, and the entry points (`Detect/`, `Type/`, `State/`, `Input/`,
//! `Language/`) sit beside them. This facade declares the modules and re-exports the
//! exact public names the rest of the crate depends on.
//!
//! The preview the model sees must be the FINAL, honest representation of the stored
//! payload. That rule is why the tree is split this finely: a detector and the arm
//! that renders its verdict have to agree, and they can only be kept in agreement if
//! the predicates they share have one definition each.

pub mod Builder;
pub mod Detect;
pub mod Detector;
pub mod Input;
#[cfg(feature = "proxy")]
pub mod Language;
pub mod Line;
pub mod State;
pub mod Text;
pub mod Type;

#[cfg(test)]
pub mod Test;

// TOML-driven preview template rendering (pre-Rust port): re-exported here so
// engine/proxy/hermes call sites keep one `aphrodite::Preview::` prefix for every
// preview knob (the `[templates.preview]` registry is consumed by `BuildPreview`
// above).
