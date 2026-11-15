//! `&str` utilities shared by the preview detectors and builders.
//!
//! Each leaf answers one question about a piece of text that several preview
//! arms need to ask - is this line structural noise, how many items precede
//! this keyword, how should an over-long line be sampled. Keeping them here
//! means a detector and the builder that renders its verdict cannot disagree
//! about what counts as a match.

pub mod EnvelopeJSON;
pub mod FirstMeaningful;
pub mod NumberBefore;
pub mod SampleLong;
