//! The mtime-cached read of one flag file.
//!
//! Prefix-tolerant on purpose: the core logger appends to the same file, so the
//! flag value can be fused with the first log line. Three-valued on purpose too:
//! "no file" and "a file that says off" are different answers, and the session
//! tree walk needs to tell them apart.

pub mod Fn;
