//! Inline-store keys for a live container.
//!
//! Both keys are derived from the content hash, so a container is reachable
//! from the marker alone: the retrieve path never needs a second index to
//! find the envelope or a chunk.

use super::super::Const::{LiveChunkSuffix, LiveMetaSuffix};

/// Inline-store key of a live container's metadata envelope.
pub(crate) fn LiveMetaKey(hash:&str) -> String { format!("{hash}{LiveMetaSuffix}") }

/// Inline-store key of a live container's `Index`-th chunk.
pub(crate) fn LiveChunkKey(hash:&str, Index:usize) -> String { format!("{hash}{LiveChunkSuffix}{Index}") }
