//! Build-output preview arm.
//!
//! A build log's whole value to the model is its tally: how many errors, how
//! many warnings, and a signal that something failed. This arm produces that
//! tally from line-shaped evidence. It is not a compiler and it resolves no
//! diagnostics - it counts the lines a compiler printed, so its honesty
//! depends entirely on counting whole lines rather than substrings.

use crate::Preview::{
	Input::Struct::Struct as Input,
	Line::{Error, Failure, Warning},
};

/// Build a preview of compiler/build output.
///
/// Promises an error count, a warning count and a failure signal, all read from
/// whole lines. It is not a substring tally: a word containing "error" must not
/// inflate the count, and a capitalised `Error:` from Python, Swift or clang
/// must not be missed. Getting this wrong is the worst failure available - a
/// genuinely failed build rendering as `0E` tells the model the build passed.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let e = Inp.raw.lines().filter(|l| Error::Fn(l)).count();
	let w = Inp.raw.lines().filter(|l| Warning::Fn(l)).count();
	// Enrich: surface the first error MESSAGE (e.g. `E0432: unresolved
	// import ...`), not just tallies - the exact text the agent needs to
	// decide whether to retrieve the full log.
	let FirstError = Inp.raw.lines().map(|l| l.trim()).find(|l| Error::Fn(l)).map(|l| {
		// Prefer the `error[EXXXX]: msg` / `error: msg` remainder.
		let Start = l
			.find("error")
			.or_else(|| l.find("Error"))
			.or_else(|| l.find("ERROR"))
			.unwrap_or(0);
		l[Start..].chars().take(60).collect::<String>()
	});
	match FirstError {
		Some(Message) if !Message.is_empty() => {
			format!("[build:{}E {}W {}L | {}]", e, w, Inp.Total, Message)
		},
		_ => {
			// Honesty: `0E 0W` renders like a clean build. When the
			// output carries a FAILURE signal (failing test run,
			// FAILED markers) but no error line was tallied, surface
			// the failure line instead of a success-looking summary -
			// a failing test run used to preview as
			// `[build:0E 0W 3L]` (ISSUE-11-PREVIEW-BATTERY #2).
			if e == 0
				&& w == 0
				&& let Some(Fail) = Inp.raw.lines().map(|l| l.trim()).find(|l| Failure::Fn(l))
			{
				return format!(
					"[build:{}E {}W {}L | {}]",
					e,
					w,
					Inp.Total,
					Fail.chars().take(60).collect::<String>()
				);
			}
			format!("[build:{}E {}W {}L]", e, w, Inp.Total)
		},
	}
}
