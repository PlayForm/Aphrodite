//! The span every terminal-output entry point opens.

/// The `hook.terminal` span, with the fields the outcome is recorded into.
///
/// One declaration for three entry points, for the same reason as the
/// tool-result span: the field list is what `Hook::Outcome::Record` fills in, and
/// a per-entry copy would let one form silently lose an annotation.
///
/// The command TEXT is deliberately absent - `command_length` identifies it
/// without putting a shell command into a log line, and a command is the most
/// likely place for a credential to be typed. `returncode` is the signal the
/// error-loop breaker keys on, so it belongs in the trace.
pub(crate) fn Fn(ContentBytes:usize, command:Option<&str>, Returncode:Option<i64>) -> tracing::Span {
	tracing::info_span!(
		"hook.terminal",
		content_bytes = ContentBytes,
		command_length = command.map(str::len).unwrap_or(0),
		returncode = Returncode.unwrap_or(0),
		compressed = tracing::field::Empty,
		outcome = tracing::field::Empty,
		ccr_type = tracing::field::Empty,
		size = tracing::field::Empty,
	)
}
