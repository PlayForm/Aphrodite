//! The Stage-2 entry point.
//!
//! Three gates, in this order, and the order matters:
//!
//! 1. **Size.** Content under `MinimumStage2Size` is never reduced - the marker
//!    and header overhead would outweigh any saving.
//! 2. **Type.** A CCR type with no registered reducer is not an error; it is
//!    the answer "this content family is already dense" (previews, listings).
//! 3. **Savings.** The reducer's output is compared against the input and
//!    discarded unless it is strictly smaller. Without this gate a reducer that
//!    finds nothing would still hand back a header, which is the one outcome
//!    that could make the model's context *larger* than the original.
//!
//! Returns `Some(reduced_string)` only when all three gates pass.

use crate::Stage2::{Const::MinimumStage2Size, Registry::Fn::ReducerRegistry};

/// Produce a semantically reduced version of content.
///
/// Returns `Some(reduced_string)` if reduction was beneficial,
/// or `None` if the content is too small, no reducer exists for the type,
/// or the reducer produced no savings.
pub fn CompressStage2(Content:&str, CCRType:&str) -> Option<String> {
	if Content.len() < MinimumStage2Size {
		return None;
	}

	let Registry = ReducerRegistry();
	let Reducer = Registry.get(CCRType)?;

	let Reduced = Reducer(Content)?;

	// Guard: don't return if reduction didn't help
	if Reduced == Content || Reduced.len() >= Content.len() {
		return None;
	}

	Some(Reduced)
}
