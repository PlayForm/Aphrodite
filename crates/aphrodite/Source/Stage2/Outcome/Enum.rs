//! The five answers Stage 2 can give.

use crate::Stage2::Reduction::Struct::Struct as Reduction;

/// Why content was or was not reduced.
///
/// Four of the five variants are reasons NOT to reduce, and they are different reasons: content
/// below the floor was never worth the parse, a type with no registered reducer is already dense
/// by design, a registered reducer that returned nothing found no structure it recognises, and a
/// reducer that ran but saved nothing would have made the model's context larger. Collapsing
/// them into `None` - which is what the previous `Option<String>` did - meant a caller could not
/// tell a misconfigured type from a dense payload.
///
/// It is not an error type: none of these is a failure, and the caller's correct response to
/// every one of them is to keep the original content.
pub(crate) enum Enum {
	/// The candidate was strictly shorter and is the replacement.
	Accepted(Reduction),
	/// Content is below `Const::MinimumStage2Size`.
	TooSmall,
	/// No reducer is registered for the CCR type.
	NoReducer,
	/// The reducer is registered but did not recognise this content.
	NotApplicable,
	/// The reducer ran and the result was not strictly shorter.
	NoSavings,
}
