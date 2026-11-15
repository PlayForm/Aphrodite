//! Error preview arm.
//!
//! Error output is the one payload where the first line is almost always the wrong line -
//! a Python traceback opens with `Traceback (most recent call last):`, a compiler log with
//! `Compiling`. This arm finds the real error line instead. It is not a stack-trace reader:
//! it surfaces one line verbatim and never reconstructs a cause chain.

use crate::Preview::{
	Input::Struct::Struct as Input,
	Line::{Error, Failure},
};

/// Characters of the hint the preview shows: sixty.
const HintShown:usize = 60;

/// Build a preview of error output.
///
/// Promises the FIRST real error line - the payload, not the traceback header. It is never
/// success-looking: when no error line is found it falls back to the last non-empty line,
/// because the tail is the most recent state. Showing the first line instead made a
/// failing Python run preview as its traceback header, which reads as noise rather than as
/// a failure.
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
		Some(Hint) => format!("[error:{}L {}B | {Hint}]", Inp.Total, Inp.bytes),
		None => format!("[error:{}L {}B]", Inp.Total, Inp.bytes),
	}
}
