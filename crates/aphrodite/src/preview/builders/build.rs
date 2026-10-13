//! build-output preview arm (honest tallies, never success-looking).

use crate::preview::{
	input::Input,
	line::{error::is_error_line, failure::is_failure_line, warning::is_warning_line},
};

/// Honest tallies (Issue #11 WS2): count error/warning LINES, not substring
/// occurrences. The old `content.matches("error").count()` inflated lines with
/// repeated occurrences, matched inside unrelated words, and missed
/// capitalized `Error:` (Python/Swift/clang output) - a genuinely failed
/// build could render as `0E`.
pub(crate) fn build_build_preview(inp:&Input<'_>) -> String {
	let e = inp.raw.lines().filter(|l| is_error_line(l)).count();
	let w = inp.raw.lines().filter(|l| is_warning_line(l)).count();
	// Enrich: surface the first error MESSAGE (e.g. `E0432: unresolved
	// import ...`), not just tallies - the exact text the agent needs to
	// decide whether to retrieve the full log.
	let first_err = inp.raw.lines().map(|l| l.trim()).find(|l| is_error_line(l)).map(|l| {
		// Prefer the `error[EXXXX]: msg` / `error: msg` remainder.
		let start = l
			.find("error")
			.or_else(|| l.find("Error"))
			.or_else(|| l.find("ERROR"))
			.unwrap_or(0);
		l[start..].chars().take(60).collect::<String>()
	});
	match first_err {
		Some(msg) if !msg.is_empty() => {
			format!("[build:{}E {}W {}L | {}]", e, w, inp.total, msg)
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
				&& let Some(fail) = inp.raw.lines().map(|l| l.trim()).find(|l| is_failure_line(l))
			{
				return format!(
					"[build:{}E {}W {}L | {}]",
					e,
					w,
					inp.total,
					fail.chars().take(60).collect::<String>()
				);
			}
			format!("[build:{}E {}W {}L]", e, w, inp.total)
		},
	}
}
