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
pub struct Struct {
	/// One entry per split part, in chain order.
	pub Segments:Vec<Segment>,
	/// Bytes of segment text the markers replace.
	pub OriginalBytes:usize,
	/// Bytes of marker text that replace them.
	pub MarkerBytes:usize,
}
