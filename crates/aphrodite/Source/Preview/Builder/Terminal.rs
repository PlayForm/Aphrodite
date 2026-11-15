//! Terminal preview arm.
//!
//! Shell output is usually a trace ending in an exit code, and the exit code is
//! the answer. This arm surfaces it. It is not a command reader: the command is
//! not reconstructed, and the arm is only reached when the classifier overrode
//! the type to `terminal`.

use crate::Preview::{Input::Struct::Struct as Input, Text::FirstMeaningful};

/// Build a preview of shell/terminal output.
///
/// Promises the exit-code or `Error:` line when one is present, otherwise the
/// FIRST meaningful line. It is not a tail reader: the last line of a trace is
/// often a bare `}` or an empty line, and previewing that made terminal output
/// render as `[terminal:7L }]` - a preview that described nothing.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let ExitLine = Inp
		.raw
		.lines()
		.rev()
		.find(|l| l.contains("exit code:") || l.contains("Error:"))
		.map(|l| l.trim());
	let FirstLine = FirstMeaningful::Fn(Inp.raw);
	let Summary = ExitLine
		.or(FirstLine.as_deref())
		.unwrap_or("")
		.chars()
		.take(60)
		.collect::<String>();
	format!("[terminal:{}L {}]", Inp.Total, Summary)
}
