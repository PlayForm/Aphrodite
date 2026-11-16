//! Derive the inline-store key of a live container's `Index`-th chunk.

use super::Const::Suffix;

/// Inline-store key of a live container's `Index`-th chunk.
///
/// Derived from the content hash plus the index, so the retrieve path needs no
/// second index to find a chunk: the marker carries the hash and the envelope
/// carries the count.
pub(crate) fn Fn(hash:&str, Index:usize) -> String { format!("{hash}{Suffix}{Index}") }
