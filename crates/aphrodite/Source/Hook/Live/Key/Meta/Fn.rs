//! Derive the inline-store key of a live container's metadata envelope.

use super::Const::Suffix;

/// Inline-store key of a live container's metadata envelope.
///
/// Derived from the content hash alone, so the retrieve path needs no second
/// index to find the envelope: the marker carries the hash.
pub(crate) fn Fn(hash:&str) -> String { format!("{hash}{Suffix}") }
