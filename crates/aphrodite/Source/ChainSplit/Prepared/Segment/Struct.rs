//! One split part's artifacts.

/// Everything one segment needs to become a retrievable marker.
///
/// `Text` is carried, not referenced, because the prepared batch outlives the
/// content string it was built from: the apply phase stores this text under
/// `Hash`, so a borrowed slice would tie the batch to the caller's buffer.
/// `Preview` and `Marker` are built once here - building them during the store
/// would do the expensive work while the session lock is held.
///
/// The fields are closed to the module because they are CORRELATED: `Hash`,
/// `Preview` and `Marker` are all derived from `Text` under one preparation
/// contract, so a segment whose hash belongs to another segment's text would be
/// a marker that resolves to the wrong content. `Prepared::Segment::Fn` is the
/// only constructor, and it derives every one of them from the same text.
pub struct Struct {
	/// Position of the part in the chain, counting from zero.
	pub(in crate::ChainSplit) Index:usize,
	/// The part's own text, exactly as it will be stored.
	pub(in crate::ChainSplit) Text:String,
	/// Reported content type of the part.
	pub(in crate::ChainSplit) Type:String,
	/// Content hash of `Text` - the store key and the marker's hash.
	pub(in crate::ChainSplit) Hash:String,
	/// The preview the model will read for this part.
	pub(in crate::ChainSplit) Preview:String,
	/// The rendered CCR marker for this part.
	pub(in crate::ChainSplit) Marker:String,
}
