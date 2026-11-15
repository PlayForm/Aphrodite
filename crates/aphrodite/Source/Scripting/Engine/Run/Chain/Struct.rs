//! The content as it moves from one script to the next.

/// What a chain of scripts has produced so far.
///
/// Two facts, and they are separate on purpose. `Content` is the value the next script
/// reads and replaces. `Touched` records whether any script returned a value at all -
/// which is NOT `Content != Payload.Content`: a script that returns its input verbatim
/// still counts as a modification, and collapsing the two would let a chain of no-op
/// scripts look like no scripting at all.
///
/// The pair used to be a `Current` string and a `Changed` boolean threaded through a
/// `for_each` with two `return`s, so the accumulator's own invariant - "these two move
/// together" - was nowhere in the types. As a struct it is one value a fold can carry.
pub(crate) struct Struct {
	/// The content the next script receives.
	pub(crate) Content:String,
	/// Whether any script has returned a value yet.
	pub(crate) Touched:bool,
}
