//! Log preview arm.
//!
//! A log's most recent state is its last line, and its most important state is whichever
//! line reports an error. Those are usually different lines. This arm prefers the error and
//! falls back to the tail. It is not a log parser: levels are not interpreted and nothing is
//! aggregated.

use crate::Preview::{
	Input::Struct::Struct as Input,
	Line::{Error, Failure},
};

/// Characters of the hint the preview shows: sixty.
const HintShown:usize = 60;

/// Build a preview of log output.
///
/// Promises an error or failure line when one exists, otherwise the last non-empty line,
/// which is the most recent state. It is not a tail dump: one line is shown, chosen so that
/// a log ending in noise can never hide the error that caused it.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let Hint = Inp
		.raw
		.lines()
		.map(|Line| Line.trim())
		.find(|Line| Error::Fn(Line) || Failure::Fn(Line))
		.or_else(|| {
			Inp.raw
				.lines()
				.rev()
				.find(|Line| !Line.trim().is_empty())
				.map(|Line| Line.trim())
		})
		.map(|Line| Line.chars().take(HintShown).collect::<String>());

	match Hint {
		Some(Hint) => format!("[log:{}L {}B | {Hint}]", Inp.Total, Inp.bytes),
		None => format!("[log:{}L {}B]", Inp.Total, Inp.bytes),
	}
}
