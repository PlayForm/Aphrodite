//! One parsed chain segment.

/// A parsed chain segment: its raw command text and its index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Struct {
	/// Position of the segment in the chain, counting from zero. The index is
	/// how a segment's output is attributed back to the command that produced
	/// it, so it must stay the original chain position even when an empty
	/// segment was dropped.
	pub Index:usize,
	/// The segment's own command text, trimmed of surrounding whitespace.
	pub command:String,
}
