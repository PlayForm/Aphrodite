//! Row normalisation: the two forms a search envelope ships its hits in.
//!
//! Hermes sends `matches` (an array of objects) or `matches_text` (a
//! path-grouped block). Both become the same `path:line:content` lines, so the
//! preview counts hits rather than trusting a number the envelope reports about
//! itself.

pub mod Array;
pub mod Text;
