//! Shape detectors, one file per SHAPE (1.5.0 REFACTOR-PLAN §3a).
//!
//! Every detector takes the shared [`Input`](crate::Preview::Input::Input)
//! and returns `bool` (the xml detector returns `Option<&'static str>`).
//! Priority = chain position in `detect.rs` - the `or_else` order.

pub mod Build;
pub mod Code;
pub mod CSV;
pub mod Diff;
pub mod Git;
pub mod GitLog;
pub mod Grep;
pub mod JSON;
pub mod Log;
pub mod Ls;
pub mod Markdown;
pub mod Search;
pub mod Table;
pub mod Terminal;
pub mod Test;
pub mod XML;
pub mod YAML;
