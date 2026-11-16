//! Key suffix of the container metadata envelope.

/// Suffix appended to the content hash to address the metadata envelope.
///
/// Wire contract: the retrieve path (`Retrieve::Live::Lookup`) builds the same
/// key from the marker's hash, so the bytes are shared state between the write
/// and the read side - change it and every existing container becomes
/// unreachable.
pub(crate) const Suffix:&str = ":live:meta";
