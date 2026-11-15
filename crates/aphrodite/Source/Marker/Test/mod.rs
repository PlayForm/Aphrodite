//! Test battery for the marker module, split by area.
//!
//! `Parse.rs` pins the reading half (normalization, validity, extraction
//! families, preview parsing); `Format.rs` pins the rendering half and the
//! doubling-bug regression; `Template.rs` pins the TOML-driven block templates
//! and `{var}` substitution.

mod Format;
mod Parse;
mod Template;
