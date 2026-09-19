//! error preview arm (surface the FIRST real error line, never the
//! traceback header, never success-looking).

use crate::preview::input::Input;
use crate::preview::line::error::is_error_line;
use crate::preview::line::failure::is_failure_line;

/// Error output (Issue #11 WS2): surface the FIRST real error line - the
/// payload, not the traceback header. The generic arm used to show the first
/// non-empty line, so a Python traceback previewed as
/// `Traceback (most recent call last):` and a compiler log as its first
/// `Compiling` line, both hiding the actual error. Never success-looking:
/// with no error line found, fall back to the last non-empty line (tail =
/// most recent state).
pub(crate) fn build_error_preview(inp: &Input<'_>) -> String {
	let hint = inp
		.raw
		.lines()
		.map(|l| l.trim())
		.find(|l| is_error_line(l) || is_failure_line(l))
		.or_else(|| inp.raw.lines().rev().find(|l| !l.trim().is_empty()).map(|l| l.trim()))
		.map(|l| l.chars().take(60).collect::<String>());
	match hint {
		Some(h) => format!("[error:{}L {}B | {}]", inp.total, inp.bytes, h),
		None => format!("[error:{}L {}B]", inp.total, inp.bytes),
	}
}
