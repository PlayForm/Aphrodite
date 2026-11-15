//! The state an admission meets.

/// What the registry looks like for one admission.
///
/// The three answers replace three separate tests a caller used to make (is this
/// identity already running, is the set full, and if so who goes), so the
/// admission path is one match over one value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Room {
	/// A running task with this identity is already tracked.
	Duplicate,
	/// There is a free slot.
	Free,
	/// The set is full; this is the victim that makes room.
	Evict(Option<usize>),
}
