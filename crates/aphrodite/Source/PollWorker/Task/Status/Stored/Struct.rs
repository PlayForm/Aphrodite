//! The identity of a task's stored output.

use crate::State::Identity::Content::Struct::Struct as Digest;

/// What a finished task records about its stored output.
///
/// One value, three fields that are always written together and read together: the
/// store key the model retrieves by, the preview it was shown, and the byte count the
/// live-container surface reports as final. The completion produces exactly this, and
/// `Status::Done`/`Failed` hold it - so the hash cannot exist without the status and
/// the status cannot exist without the hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Struct {
	/// CCR hash of the stored output - the store key and the marker's hash.
	pub Hash:Digest,
	/// The preview the marker and the completion nudge show.
	pub Preview:String,
	/// Byte size of the stored output.
	pub Size:usize,
}
