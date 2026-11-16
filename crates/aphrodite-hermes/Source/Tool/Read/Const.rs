//! The hard size ceiling on a direct path read.
//!
//! A separate leaf from the read so the number has one home: the error message
//! quotes it and the comparison uses it, and a copy that drifted would report a
//! cap it does not enforce.

/// Largest file `aphrodite_retrieve(path=...)` will read directly.
///
/// 10 MiB. It is a ceiling, not a target: a larger file is REFUSED rather than
/// truncated, because a silently shortened read looks like a complete file to the
/// model and would be trusted as one.
pub(crate) const CAP:u64 = 10 * 1024 * 1024;
