//! The preview length budget.
//!
//! The configured `preview_max_chars` is a `usize` whose `0` means "no cap", which
//! is the one reading the language gives nothing. The enum is that reading made
//! explicit, so no consumer has to remember which sentinel applies - and the
//! translation from the configured number happens in exactly one place.

pub mod Enum;
pub mod Resolve;
