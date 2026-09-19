//! Shape detectors, one file per SHAPE (1.5.0 REFACTOR-PLAN §3a).
//!
//! Every detector takes the shared [`Input`](crate::preview::input::Input)
//! and returns `bool` (the xml detector returns `Option<&'static str>`).
//! Priority = chain position in `detect.rs` - the `or_else` order.

pub mod build;
pub mod code;
pub mod csv;
pub mod diff;
pub mod git;
pub mod gitlog;
pub mod grep;
pub mod json;
pub mod log;
pub mod ls;
pub mod markdown;
pub mod search;
pub mod table;
pub mod terminal;
pub mod test;
pub mod xml;
pub mod yaml;
