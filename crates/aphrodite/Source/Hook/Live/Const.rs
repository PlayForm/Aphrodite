//! Live-container store constants.

/// Target byte size of one live-container chunk. Chunking cuts on UTF-8
/// char boundaries, so a chunk may be a few bytes under this when a
/// multi-byte character straddles the boundary.
pub(crate) const LiveChunkSize:usize = 1024;
/// Inline-store key suffix of the container metadata envelope.
pub(crate) const LiveMetaSuffix:&str = ":live:meta";
/// Inline-store key segment prefix of the per-chunk entries.
pub(crate) const LiveChunkSuffix:&str = ":live:chunk:";
