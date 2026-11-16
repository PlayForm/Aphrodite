//! The per-file prefetch size cap.
//!
//! One number, in its own file, because two different leaves enforce it: the
//! read pass skips a file over the cap, and the insert pass quotes the same
//! cap back in the skip reason. A second literal would let the message and
//! the check disagree, which reads to a caller as a bug in the reader.

/// Maximum file size for prefetch (10MB).
pub(crate) const Maximum:u64 = 10 * 1024 * 1024;
