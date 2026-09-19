//! linter preview arm (surface the first issue line).

use crate::preview::input::Input;
use crate::preview::line::lint::is_lint_line;

/// Linter output (ruff/eslint/clippy/flake8): surface the first issue line
/// (`path:line:col: CODE message`, or `error:`/`warning:` lines).
pub(crate) fn build_lint_preview(inp: &Input<'_>) -> String {
	let hint = inp
		.raw
		.lines()
		.map(|l| l.trim())
		.find(|l| is_lint_line(l))
		.or_else(|| inp.raw.lines().rev().find(|l| !l.trim().is_empty()).map(|l| l.trim()))
		.map(|l| l.chars().take(60).collect::<String>());
	match hint {
		Some(h) => format!("[lint:{}L {}B | {}]", inp.total, inp.bytes, h),
		None => format!("[lint:{}L {}B]", inp.total, inp.bytes),
	}
}
