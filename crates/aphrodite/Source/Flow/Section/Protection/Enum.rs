//! Whether the budget may drop a section.

/// A section's protection under budget pressure.
///
/// The assembler used to hand the budget a bare count of "leading entries that
/// must survive", which relied on three separate facts staying true: the vector
/// was ordered, the count matched the sections that were present, and the
/// protected ones occupied the prefix. Protection travels with the section now,
/// so the floor is derived from the sections themselves and cannot drift from
/// the order they were collected in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enum {
	/// Never dropped: the session is following this text, so losing it is a
	/// correctness failure rather than a size optimisation.
	Required,
	/// Dropped from the bottom of the candidate list when the turn is too big.
	Optional,
}
