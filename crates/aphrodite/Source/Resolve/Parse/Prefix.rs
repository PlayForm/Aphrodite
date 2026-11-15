//! The marker opening delimiter.
//!
//! One constant, because the spelling of `<<<CCR:` is a wire contract: a change
//! here stops every already-emitted marker from resolving, and stops the scanner
//! from finding markers inside content that was written before the change. It is
//! not a regex and it carries no escape rules - the grammar has none, so a marker
//! is found by literal search.
//!
//! Named `Const` because the file name carries the domain, per the house style.
#![allow(non_upper_case_globals)]

/// The text every CCR marker starts with.
///
/// Shared by the scanner and the hash extractor so the two can never disagree
/// about where a marker begins. It is not configurable and must not become so:
/// markers already written into stored content and into conversation history
/// carry this exact prefix, and a runtime change would strand them.
pub(crate) const Const:&str = "<<<CCR:";
