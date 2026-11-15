//! Terminal preview arm.
//!
//! Shell output is usually a trace ending in an exit code, and the exit code is the answer.
//! This arm surfaces it. It is not a command reader: the command is not reconstructed, and
//! the arm is only reached when the classifier overrode the type to `terminal`.

use crate::Preview::{Input::Struct::Struct as Input, Text::FirstMeaningful};

/// Characters of the summary the preview shows: sixty.
const SummaryShown:usize = 60;

/// Build a preview of shell/terminal output.
///
/// Promises the exit-code or `Error:` line when one is present, otherwise the FIRST
/// meaningful line. It is not a tail reader: the last line of a trace is often a bare `}`
/// or an empty line, and previewing that made terminal output render as `[terminal:7L }]` -
/// a preview that described nothing.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let Exit = Inp
		.raw
		.lines()
		.rev()
		.find(|Line| Line.contains("exit code:") || Line.contains("Error:"))
		.map(|Line| Line.trim());
	let Summary = Exit
		.or(FirstMeaningful::Fn(Inp.raw).as_deref())
		.unwrap_or("")
		.chars()
		.take(SummaryShown)
		.collect::<String>();

	format!("[terminal:{}L {Summary}]", Inp.Total)
}
