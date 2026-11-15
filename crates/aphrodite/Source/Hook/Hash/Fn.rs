//! The CCR content hash.

/// Compute a CCR hash for content using BLAKE3 (40 hex chars).
///
/// This is the only hash the compression path uses: the marker's hash, the
/// inline-store key, the live-container key prefix and the `AphroditeRetrieve`
/// lookup all derive from it. Changing the function would orphan every entry
/// already stored in a session, so it is a wire contract, not an internal
/// detail.
pub fn Fn(Content:&str) -> String { headroom_core::ccr::compute_key(Content.as_bytes()) }
