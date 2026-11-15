//! The append itself.
//!
//! `Fn/` is the record writer at the process-global level, `At/` is the same
//! writer at an explicit level, `Verbose/` is the level-2-only variant, and
//! `Sanitize/` is the one-line guarantee that a value cannot break the record.
//! Every writer funnels into `At/`, so the file format has exactly one
//! implementation.

pub mod At;
pub mod Fn;
pub mod Sanitize;
pub mod Verbose;
