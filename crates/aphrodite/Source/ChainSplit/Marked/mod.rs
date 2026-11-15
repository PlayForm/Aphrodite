//! The two halves of the marker rewrite: writing a marked command and reading
//! its marked output back apart.
//!
//! They live together because they are one format seen from both ends - the
//! string `Command/` emits is exactly what `Output/` parses, so a change to
//! either is a change to both.

pub mod Command;
pub mod Output;
