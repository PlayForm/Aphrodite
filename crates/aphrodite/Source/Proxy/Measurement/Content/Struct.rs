//! What one rewrite did to the CONTENT it replaced.
//!
//! The unit here is stored content: the length of the message content before the
//! rewrite, and the length of the marker that replaced it. That is the number
//! the model's context shrinks by, and it is not the same measurement as the
//! encoded HTTP body - a JSON string's serialised length differs from its
//! content length (escaping, surrounding envelope), and a cache hit avoids a
//! whole body without replacing any content. The two used to be added to one
//! counter, which left it describing neither.
//!
//! Savings used to be counted at the moment a marker was rendered, which is not
//! the same thing as the bytes that left the proxy: if the transformed body then
//! failed to serialise and the original was emitted instead, the counter claimed
//! a saving that never happened. Carrying the accounting inside the final-body
//! value means the counter can only be incremented from the body that was
//! actually sent.

/// Content bytes in, content bytes out, for one rewrite.
#[derive(Clone, Copy)]
pub(crate) struct Struct {
	/// The content length before the rewrite.
	pub(crate) OriginalBytes:usize,
	/// The content length after it - the marker's rendered length.
	pub(crate) EmittedBytes:usize,
}

impl Struct {
	/// Content bytes removed by the rewrite, saturating at zero.
	pub(crate) fn Saved(&self) -> u64 { self.OriginalBytes.saturating_sub(self.EmittedBytes) as u64 }
}
