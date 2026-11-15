//! What the retention pass decided about one task.

/// The fate of one task in this expiry call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetentionDecision {
	/// Nothing changes.
	Keep,
	/// The agent stopped polling it: mark it abandoned, keep the record.
	MarkStale,
	/// Its lifetime has run out: drop the record.
	Remove,
}
