//! One split part's artifacts.

/// Everything one segment needs to become a retrievable marker.
///
/// `Text` is carried, not referenced, because the prepared batch outlives the
/// content string it was built from: the apply phase stores this text under
/// `Hash`, so a borrowed slice would tie the batch to the caller's buffer.
/// `Preview` and `Marker` are built once here - building them during the store
/// would do the expensive work while the session lock is held.
pub struct Struct {
	/// Position of the part in the chain, counting from zero.
	pub Index:usize,
	/// The part's own text, exactly as it will be stored.
	pub Text:String,
	/// Reported content type of the part.
	pub Type:String,
	/// Content hash of `Text` - the store key and the marker's hash.
	pub Hash:String,
	/// The preview the model will read for this part.
	pub Preview:String,
	/// The rendered CCR marker for this part.
	pub Marker:String,
}
