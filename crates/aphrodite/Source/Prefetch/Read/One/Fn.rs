//! Read one path, in the guard's order.

use std::path::Path;

use crate::Prefetch::{
	Outcome::Enum::Enum as Outcome,
	Read::{Content::Fn::Fn as ReadContent, Refuse::Fn::Fn as Refuse},
};

/// Classify one path: refused, missing, oversize, unreadable, or loaded.
///
/// The rules are a chain of alternatives rather than a sequence of early returns,
/// because each one either answers or gets out of the way and the FIRST answer is
/// the outcome. The order is the guard's: a sensitive path is refused before it is
/// stat'ed, a non-file is missing before its size is read, and an unreadable path
/// falls through to `Error` - which is also the answer when `metadata` fails, so
/// the chain's default is the error case rather than a fabricated success.
pub(crate) fn Fn(PathStr:&str) -> Outcome {
	let path = Path::new(PathStr);

	Refuse(path)
		.map(|Reason| Outcome::Refused { Reason })
		.or_else(|| (!path.is_file()).then_some(Outcome::Missing))
		.or_else(|| ReadContent(path))
		.unwrap_or(Outcome::Error)
}
