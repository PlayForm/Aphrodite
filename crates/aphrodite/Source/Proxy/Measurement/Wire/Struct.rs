//! What one rewrite did to the bytes on the wire.
//!
//! Savings used to be added to the counter at the moment a marker was rendered,
//! which is not the same thing as the bytes that left the proxy: if the
//! transformed body then failed to serialise and the original was emitted
//! instead, the counter claimed a saving that never happened. Carrying the
//! accounting inside the final-body value means the counter can only be
//! incremented from the body that was actually sent.

/// Bytes in, bytes out, for one rewrite.
pub(crate) struct Struct {
	/// The content length before the rewrite.
	pub(crate) OriginalBytes:usize,
	/// The content length after it - the marker's rendered length.
	pub(crate) EmittedBytes:usize,
}

impl Struct {
	/// Bytes removed by the rewrite, saturating at zero.
	pub(crate) fn Saved(&self) -> u64 { self.OriginalBytes.saturating_sub(self.EmittedBytes) as u64 }
}
