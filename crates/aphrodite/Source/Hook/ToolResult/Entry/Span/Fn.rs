//! The span every tool-result entry point opens.

/// The `hook.tool_result` span, with the fields the outcome is recorded into.
///
/// One declaration for three entry points: the plain, the classified and the
/// meta-carrying form all feed the same pipeline, and a per-entry copy of this
/// field list would let one of them lose an annotation without anyone noticing.
///
/// `tool` and `content_bytes` identify the work; `compressed`, `outcome`,
/// `ccr_type` and `size` start `Empty` and are filled by `Hook::Outcome::Record`
/// once the pipeline has decided. Nothing here is content: the payload itself
/// never enters a log line.
pub(crate) fn Fn(ToolName:&str, ContentBytes:usize) -> tracing::Span {
	tracing::info_span!(
		"hook.tool_result",
		tool = %ToolName,
		content_bytes = ContentBytes,
		compressed = tracing::field::Empty,
		outcome = tracing::field::Empty,
		ccr_type = tracing::field::Empty,
		size = tracing::field::Empty,
	)
}
