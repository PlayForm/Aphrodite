//! log preview arm (error signal wins, else the tail).

use crate::preview::{
	input::Input,
	line::{error::is_error_line, failure::is_failure_line},
};

/// Log output: the LAST non-empty line is the most recent state, and an
/// error/failure line (if any) is the signal that matters - prefer it over
/// the tail so a log ending in noise never hides the error.
pub(crate) fn build_log_preview(inp:&Input<'_>) -> String {
	let hint = inp
		.raw
		.lines()
		.map(|l| l.trim())
		.find(|l| is_error_line(l) || is_failure_line(l))
		.or_else(|| inp.raw.lines().rev().find(|l| !l.trim().is_empty()).map(|l| l.trim()))
		.map(|l| l.chars().take(60).collect::<String>());
	match hint {
		Some(h) => format!("[log:{}L {}B | {}]", inp.total, inp.bytes, h),
		None => format!("[log:{}L {}B]", inp.total, inp.bytes),
	}
}
