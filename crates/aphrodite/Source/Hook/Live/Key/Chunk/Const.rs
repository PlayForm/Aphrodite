//! Key segment prefix of the per-chunk entries.

/// Prefix appended to the content hash, before the chunk index.
///
/// Wire contract: the retrieve path (`Retrieve::Live::Lookup`) rebuilds the same
/// key from the marker's hash and index, so the bytes are shared state between
/// the write and the read side.
pub(crate) const Suffix:&str = ":live:chunk:";
