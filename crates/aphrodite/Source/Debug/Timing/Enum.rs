//! The timing categories.

/// Timing categories averaged in the metrics snapshot.
///
/// The discriminants index the accumulator arrays, exactly like the counter
/// enum: renumbering a variant moves its samples onto another category and the
/// snapshot then reports a plausible but wrong average.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enum {
	/// Tool/terminal transform hook duration.
	ToolTransform = 0,
	/// Retrieval (`Resolve::Expand::Fn`) duration.
	Retrieve = 1,
	/// `Flow::build_turn_context` duration.
	PreTurn = 2,
	/// Template render duration.
	Render = 3,
}
