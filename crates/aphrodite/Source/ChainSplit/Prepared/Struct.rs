//! A whole chain's artifacts, built before any session state is touched.

use super::Segment::Struct::Struct as Segment;

/// Every artifact the split needs, and nothing that is not one.
///
/// It is a value rather than a sequence of stores because preparation and
/// application are separate decisions: the split is decided from the content
/// alone, and only a prepared batch is worth mutating state for. The two byte
/// totals are artifacts in their own right - the summary line the caller emits
/// is built from them, and recomputing them after the store would describe the
/// markers rather than the split.
///
/// The fields are closed to the module: a batch is built by [`Struct::Assemble`]
/// and consumed by `Prepared::Apply`, so no other code can fabricate a batch
/// whose totals disagree with its segments, or hand the same batch to two
/// apply calls. `Hook::Terminal::Plan::Split` carries it as an opaque token for
/// exactly that reason.
pub struct Struct {
	/// One entry per split part, in chain order.
	pub(in crate::ChainSplit) Segments:Vec<Segment>,
	/// Bytes of segment text the markers replace.
	pub(in crate::ChainSplit) OriginalBytes:usize,
	/// Bytes of marker text that replace them.
	pub(in crate::ChainSplit) MarkerBytes:usize,
}

impl Struct {
	/// Assemble a batch from its segments, deriving the byte totals.
	///
	/// `None` is "not a chain": fewer than two non-empty parts means there is
	/// nothing to split, and a batch of one would be a marker that claims a
	/// split happened. Deriving the totals here is what makes them true by
	/// construction - there is no constructor that takes them from a caller.
	pub(in crate::ChainSplit) fn Assemble(Segments:Vec<Segment>) -> Option<Struct> {
		(Segments.len() >= 2).then(|| {
			let OriginalBytes = Segments.iter().map(|Segment| Segment.Text.len()).sum();
			let MarkerBytes = Segments.iter().map(|Segment| Segment.Marker.len()).sum();

			Struct { Segments, OriginalBytes, MarkerBytes }
		})
	}
}
