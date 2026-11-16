//! Target byte size of one live-container chunk.

/// Target byte size of one live-container chunk.
///
/// Chunking cuts on UTF-8 char boundaries, so a chunk may be a few bytes under
/// this when a multi-byte character straddles the boundary. Zero is not a valid
/// value: a policy built from it yields no chunks, and the install pass then
/// degrades the container to a plain entry.
pub(crate) const Size:usize = 1024;
