//! The encoded response body a cache hit avoided.
//!
//! A hit skips an entire upstream round-trip, so the whole serialised body is
//! bytes that never crossed the network. That is bandwidth, measured on the
//! encoded body - it is not content savings, which are measured on the stored
//! content a marker replaced (`Measurement::Content`). The two are reported
//! separately because a hit and a rewrite answer different questions: "how much
//! context did the model not have to hold" versus "how many bytes did the wire
//! not carry".

/// The encoded body one avoided upstream round-trip would have carried.
#[derive(Clone, Copy)]
pub(crate) struct Struct {
	/// The serialised response body's length.
	pub(crate) BodyBytes:usize,
}

impl Struct {
	/// The encoded bytes that were never fetched.
	pub(crate) fn Avoided(&self) -> u64 { self.BodyBytes as u64 }
}
