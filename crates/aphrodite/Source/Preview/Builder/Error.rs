//! Error preview arm.
//!
//! Error output is the one payload where the first line is almost always the
//! wrong line - a Python traceback opens with `Traceback (most recent call
//! last):`, a compiler log with `Compiling`. This arm finds the real error line
//! instead. It is not a stack-trace reader: it surfaces one line verbatim and
//! never reconstructs a cause chain.

use crate::Preview::{
	Input::Struct::Struct as Input,
	Line::{Error, Failure},
};

/// Build a preview of error output.
///
/// Promises the FIRST real error line - the payload, not the traceback header.
/// It is never success-looking: when no error line is found it falls back to the
/// last non-empty line, because the tail is the most recent state. Showing the
/// first line instead made a failing Python run preview as its traceback
/// header, which reads as noise rather than as a failure.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let Hint = Inp
		.raw
		.lines()
		.map(|l| l.trim())
		.find(|l| Error::Fn(l) || Failure::Fn(l))
		.or_else(|| Inp.raw.lines().rev().find(|l| !l.trim().is_empty()).map(|l| l.trim()))
		.map(|l| l.chars().take(60).collect::<String>());
	match Hint {
		Some(h) => format!("[error:{}L {}B | {}]", Inp.Total, Inp.bytes, h),
		None => format!("[error:{}L {}B]", Inp.Total, Inp.bytes),
	}
}
