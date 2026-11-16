//! What budget enforcement established.

use crate::Flow::TurnContext::Budget::Measured::Struct::Struct as Measured;

/// The result of fitting the candidate sections into the turn's budget.
///
/// Two outcomes, because the configured limit cannot always be honoured: the
/// budget is measured in BYTES while `flow_budget_chars` reads as characters,
/// and the protected sections are never dropped. A turn whose required sections
/// alone exceed the budget therefore returns oversized text. That used to happen
/// silently, which turned "hard cap" into a documentation contradiction; as a
/// result type it is something the caller has to handle, and the assembler
/// handles it by keeping the required text and reporting the overflow.
pub(crate) enum Enum {
	/// The surviving sections fit the budget.
	Fits(Measured),
	/// The protected sections alone exceed the budget: they are preserved, and
	/// the conflict is reported rather than truncated away.
	ProtectedOverflow {
		/// What survived - all of it required.
		Surviving:Measured,
		/// The configured byte limit.
		Limit:usize,
		/// How many bytes over the limit the required sections are.
		Excess:usize,
	},
}
