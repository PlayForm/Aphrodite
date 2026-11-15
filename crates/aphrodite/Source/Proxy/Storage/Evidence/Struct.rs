//! The content whose storage was verified, under the hash it landed at.
//!
//! The fields are `pub(in crate::Proxy::Storage)`, not `pub(crate)`: only
//! `Storage::Store` and `Storage::Put` - the two operations that actually talk
//! to the backend - can build one, so the existence of this value at a call site
//! is the proof that the store accepted the write. A struct with public fields
//! would be a receipt anyone could print.
//!
//! It carries the content so a marker is rendered from the bytes that were
//! stored rather than from a copy that may already have diverged, and the hash
//! so the marker names the key that resolves.

/// Verified storage of one piece of content.
pub(crate) struct Struct {
	/// The hash the content is readable under.
	pub(in crate::Proxy::Storage) Hash:String,
	/// The content that was stored.
	pub(in crate::Proxy::Storage) Content:String,
}

impl Struct {
	/// The hash the content is readable under.
	pub(crate) fn Hash(&self) -> &str { &self.Hash }

	/// The stored content, as the store returned it.
	pub(crate) fn Content(&self) -> &str { &self.Content }

	/// The stored content's length in bytes.
	pub(crate) fn Size(&self) -> usize { self.Content.len() }
}
