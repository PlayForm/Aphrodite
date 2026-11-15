//! The append itself.
//!
//! `Fn/` is the record writer at the process-global level, `At/` is the same
//! writer at an explicit level, `Verbose/` is the level-2-only variant,
//! `Sanitize/` is the one-line guarantee that a value cannot break the record,
//! and `Emit/` is the same record handed to the `tracing` stack. Every writer
//! funnels into `At/`, so the file format has exactly one implementation and
//! the two destinations can never disagree about what happened.

pub mod At;
pub mod Emit;
pub mod Fn;
pub mod Sanitize;
pub mod Verbose;
