//! terminal preview arm (exit-code signal, first meaningful line).

use crate::preview::input::Input;
use crate::preview::text::first_meaningful::first_meaningful_line;

/// `hooks::transform_terminal_output` overrides the classified type to
/// "terminal" when the content looks like a shell/exit-code trace; this arm
/// surfaces the exit-code / `Error:` line when present (most recent state
/// signal), else the FIRST meaningful line (skipping lone braces - the
/// `[terminal:7L }]` bug class).
pub(crate) fn build_terminal_preview(inp:&Input<'_>) -> String {
	let exit_line = inp
		.raw
		.lines()
		.rev()
		.find(|l| l.contains("exit code:") || l.contains("Error:"))
		.map(|l| l.trim());
	let first_line = first_meaningful_line(inp.raw);
	let summary = exit_line
		.or(first_line.as_deref())
		.unwrap_or("")
		.chars()
		.take(60)
		.collect::<String>();
	format!("[terminal:{}L {}]", inp.total, summary)
}