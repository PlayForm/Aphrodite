//! The three gates, as a value.

use crate::Stage2::{
	Const::MinimumStage2Size,
	Outcome::Enum::Enum as Outcome,
	Reduction::Accept::Fn::Fn as Accept,
	Registry::Fn::ReducerRegistry,
};

/// Decide what Stage 2 does with a piece of content.
///
/// Three gates, in this order, and the order matters:
///
/// 1. **Size.** Content under `MinimumStage2Size` is never reduced - the marker and header overhead
///    would outweigh any saving.
/// 2. **Type.** A CCR type with no registered reducer is not an error; it is the answer "this content
///    family is already dense" (previews, listings).
/// 3. **Savings.** The reducer's output is compared against the input and discarded unless it is
///    strictly smaller. Without this gate a reducer that finds nothing would still hand back a header,
///    which is the one outcome that could make the model's context larger than the original.
///
/// It is not a reducer: it runs one and judges the result, so a reducer can never widen its own
/// acceptance. The size gate is a `let ... else` on `checked_sub` because "there is headroom" is a
/// value, not a branch worth spelling.
pub(crate) fn Fn(Content:&str, CCRType:&str) -> Outcome {
	let Some(_) = Content.len().checked_sub(MinimumStage2Size) else { return Outcome::TooSmall };

	let Reducers = ReducerRegistry();
	let Reducer = match Reducers.get(CCRType) {
		Some(Reducer) => Reducer,
		None => return Outcome::NoReducer,
	};

	let Reduced = match Reducer(Content) {
		Some(Reduced) => Reduced,
		None => return Outcome::NotApplicable,
	};

	match Accept(Reduced, Content.len()) {
		Some(Reduction) => Outcome::Accepted(Reduction),
		None => Outcome::NoSavings,
	}
}
