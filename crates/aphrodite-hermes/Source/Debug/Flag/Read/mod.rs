//! The mtime-cached read of one flag file.
//!
//! Prefix-tolerant on purpose: the core logger appends to the same file, so the
//! flag value can be fused with the first log line.

pub mod Fn;
