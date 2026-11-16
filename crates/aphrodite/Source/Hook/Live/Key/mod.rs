//! Inline-store keys for a live container.
//!
//! Both keys are derived from the content hash, so a container is reachable from
//! the marker alone: the retrieve path never needs a second index to find the
//! envelope or a chunk. `Meta/` addresses the metadata envelope, `Chunk/`
//! addresses one numbered chunk.

pub mod Chunk;
pub mod Meta;
