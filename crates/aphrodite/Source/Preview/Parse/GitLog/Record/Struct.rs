//! One complete git-log record.

/// A commit reduced to the two facts the preview shows.
///
/// `Subject` is held whole - the 32-character cut is a display decision, so it belongs
/// to the renderer, not here. It is not a history entry: parents, branches and merges
/// are not reconstructed, and an empty subject is a real value (a commit with no
/// message) rather than a missing one.
pub(crate) struct Struct {
	pub Hash:String,
	pub Subject:String,
}
