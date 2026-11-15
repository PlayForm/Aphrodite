//! The command-prefix table and the matcher that consults it.
//!
//! This is the only place that knows which commands are presumed slow. It is a
//! denylist of known-slow prefixes, not a timer: an unknown command is never
//! backgrounded however large its output, because backgrounding something that has
//! already finished would hand the agent a marker to poll for work that is done.

pub mod Fn;
pub mod Prefix;
