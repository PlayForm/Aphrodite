//! The mtime-cached read of one flag file.
//!
//! Prefix-tolerant on purpose: the core logger appends to the same file, so the
//! flag value can be fused with the first log line.

mod Fn;

pub(crate) use Fn::Fn;
