//! The line an outcome carries, when it carries one.

use crate::Setup::MacOS::Outcome;

/// The warning this outcome wants printed, if any.
pub fn Fn(Outcome:Outcome::Enum::Enum) -> Option<String> {
	match Outcome {
		Outcome::Enum::Enum::Done => None,
		Outcome::Enum::Enum::Warned(Warning) => Some(Warning),
	}
}
