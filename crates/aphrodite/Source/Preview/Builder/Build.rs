//! Build-output preview arm.
//!
//! A build log's whole value to the model is its tally: how many errors, how many
//! warnings, and a signal that something failed. This arm produces that tally from
//! line-shaped evidence. It is not a compiler and it resolves no diagnostics - it counts
//! the lines a compiler printed, so its honesty depends entirely on counting whole lines
//! rather than substrings.

use crate::Preview::{
	Input::Struct::Struct as Input,
	Line::{Error, Failure, Warning},
};

/// Characters of a diagnostic message the preview shows: sixty.
const MessageShown:usize = 60;

/// Build a preview of compiler/build output.
///
/// Promises an error count, a warning count and a failure signal, all read from whole
/// lines. It is not a substring tally: a word containing "error" must not inflate the
/// count, and a capitalised `Error:` from Python, Swift or clang must not be missed.
/// Getting this wrong is the worst failure available - a genuinely failed build rendering
/// as `0E` tells the model the build passed.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let Errors = Inp.raw.lines().filter(|Line| Error::Fn(Line)).count();
	let Warnings = Inp.raw.lines().filter(|Line| Warning::Fn(Line)).count();
	// Enrich: surface the first error MESSAGE (e.g. `E0432: unresolved import ...`),
	// not just tallies - the exact text the agent needs to decide whether to retrieve
	// the full log.
	let FirstError = Inp
		.raw
		.lines()
		.map(|Line| Line.trim())
		.find(|Line| Error::Fn(Line))
		.map(|Line| {
			// Prefer the `error[EXXXX]: msg` / `error: msg` remainder.
			let Start = Line
				.find("error")
				.or_else(|| Line.find("Error"))
				.or_else(|| Line.find("ERROR"))
				.unwrap_or(0);

			Line[Start..].chars().take(MessageShown).collect::<String>()
		})
		.filter(|Message| !Message.is_empty());

	match FirstError {
		Some(Message) => format!("[build:{Errors}E {Warnings}W {}L | {Message}]", Inp.Total),
		None => {
			// Honesty: `0E 0W` renders like a clean build. When the output carries a
			// FAILURE signal (failing test run, FAILED markers) but no error line was
			// tallied, surface the failure line instead of a success-looking summary - a
			// failing test run used to preview as `[build:0E 0W 3L]`.
			let Fail = if Errors == 0 && Warnings == 0 {
				Inp.raw.lines().map(|Line| Line.trim()).find(|Line| Failure::Fn(Line))
			} else {
				None
			};

			match Fail {
				Some(Fail) => {
					format!(
						"[build:{Errors}E {Warnings}W {}L | {}]",
						Inp.Total,
						Fail.chars().take(MessageShown).collect::<String>()
					)
				},
				None => format!("[build:{Errors}E {Warnings}W {}L]", Inp.Total),
			}
		},
	}
}
