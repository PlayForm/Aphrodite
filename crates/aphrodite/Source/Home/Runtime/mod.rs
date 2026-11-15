//! The runtime home - the Aphrodite user-data folder.
//!
//! Same two-answer split as `Hermes/`: `Fn/` degrades to `.`, `Opt/` reports
//! `None`. The precedence chain itself lives in `Opt/Fn.rs` and is the single
//! decision the whole crate reads; nothing else may re-derive it.

pub mod Fn;
pub mod Opt;
