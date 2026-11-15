//! The terminal envelope: `{"output": "...", "exit_code": N, ...}`.
//!
//! First in the chain by position, because a terminal result that also carries
//! an `error` string must classify by its output when the output is non-empty.
//! The empty-output case deliberately falls through to the `error` matcher
//! instead of aborting: a failed command with empty stdout IS its error text.

pub mod Classify;
pub mod Fn;
