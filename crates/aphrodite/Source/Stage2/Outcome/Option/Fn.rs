//! The public `Option<String>` view of an outcome.

use crate::Stage2::Outcome::Enum::Enum;

/// Keep only the accepted reduction.
///
/// This is the adapter the public entry point uses, because its callers genuinely do not need
/// the reasons - they either replace the content or leave it alone. It is not the internal
/// representation: anything that wants to know WHY it did not reduce reads `Enum` directly.
pub(crate) fn Fn(Outcome:Enum) -> Option<String> {
	match Outcome {
		Enum::Accepted(Reduction) => Some(Reduction.Take()),
		Enum::TooSmall | Enum::NoReducer | Enum::NotApplicable | Enum::NoSavings => None,
	}
}
