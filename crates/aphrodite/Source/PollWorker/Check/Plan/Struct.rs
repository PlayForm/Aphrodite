//! A nudge, rendered but not yet pushed.
//!
//! Text and lifetime travel together because they are one decision: a completion
//! announcement lives two turns so the agent has time to retrieve, a running-task
//! reminder lives one, and a plan that carried only its text would let the two
//! drift apart.

/// One nudge, ready to queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Struct {
	/// The sentence the model reads.
	pub Text:String,
	/// Turns the nudge survives in the directive queue.
	pub TtlTurns:usize,
}
