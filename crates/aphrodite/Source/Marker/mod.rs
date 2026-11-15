//! CCR marker generation - 1:1 port of plugins/aphrodite/_marker/marker.py
//!
//! Generates <<<CCR:hash|type|size>>> markers with TOML-driven templates.
//!
//! The marker line is a wire format: the LLM reads it and the resolver parses
//! it, so the grammar (`<<<CCR:{hash}|{type}|{size}>>>`) and the preview line
//! under it are a contract, not a rendering choice. Anything that changes a
//! byte of that output changes what every already-emitted marker means.
//!
//! The tree separates the two halves - producing a marker and reading one back
//! - because they fail in opposite directions:
//!
//! - `Format/` - rendering: the entry point, the template dispatch, the fixed
//!   fallback grammar, the "already bracketed?" test, `{var}` substitution, and
//!   the process-global template registration.
//! - `Parse/` - reading: hash normalization, hash validity, preview extraction,
//!   and the multi-family hash extractor.
//! - `Test/` - the battery, split by area.
//!
//! This facade re-exports the exact public names the rest of the crate (and
//! `aphrodite-hermes` via `Library.rs`) depends on.

mod Format;
mod Parse;

#[cfg(test)]
mod Test;

pub use Format::{Const::SetMarkerTemplates, Entry::CCRMarker};
pub use Parse::{Extract::ExtractHashes, Normalize::NormalizeHash, Preview::ParsePreview, Valid::IsValidCCRHash};
pub(crate) use Format::Vars::SubstituteVars;
#[cfg(test)]
pub(crate) use Format::Const::{MarkerHintEnabled, MarkerTemplatesTestGuard, RegisteredMarkerFormat};
