//! Classify proxy content into the type names the CCR layer and the previews
//! understand.
//!
//! Deliberately string-based and order-sensitive: the first matching family
//! wins, and the returned names are the same ones `ThresholdFor` scales on, so
//! a new name here without a matching arm there silently falls to the base
//! threshold.

/// Detect content type for adaptive compression strategy.
///
/// 1.5.0 REFACTOR-PLAN §7: the old 145-line parallel classifier is gone -
/// this is now a THIN call site over the single detection pipeline
/// (`crate::Preview::detect_semantic_type`) with the per-language code
/// refinement (`preview/lang.rs`). Envelope JSON (`ExitCode`/`Status`) is
/// deliberately excluded from `JSON` by the pipeline's envelope guard (the
/// raw-JSON preview is the payload, WS1); the proxy keeps that one explicit
/// check so tool envelopes still get the tool_output arm.
pub(crate) fn Fn(Content:&str) -> &'static str {
	// tool_output envelope: the pipeline's json detector + envelope guard
	// exclude Hermes wrapper envelopes from `JSON` on purpose (raw-JSON
	// preview is the payload); recognize them here so the stored type still
	// routes to the tool_output alias arm on the json builder.
	if Content.starts_with('{') || Content.starts_with('[') {
		if serde_json::from_str::<serde_json::Value>(Content).is_err() {
			return "text";
		}
		if Content.contains("exit_code") || Content.contains("\"status\"") {
			return "tool_output";
		}
		return "json";
	}
	match crate::Preview::DetectSemanticType(Content) {
		// Language refinement: shape "code" → code_rust/python/go/js.
		Some("code") => crate::Preview::Language::Fn::Fn(Content).unwrap_or("code"),
		Some(t) => t,
		// Proxy-path glue: the pipeline deliberately leaves error/linter/
		// build_output hint-supplied (the caller hint or classifier supplies
		// them), but this gateway path HAS no hint. Keep the first-line
		// recognition so un-hinted tracebacks/lint dumps/build logs keep
		// their honest arms (WS2 - a traceback previewing as `[text:... |
		// Traceback (most recent call last):]` is the misleading-preview bug
		// class). diff/git/gitlog/log are already pipeline detectors.
		None => {
			let FirstLine = Content.lines().next().unwrap_or("");
			if FirstLine.starts_with("error")
				|| FirstLine.starts_with("Error")
				|| FirstLine.starts_with("ERROR")
				|| FirstLine.starts_with("Traceback")
				|| FirstLine.starts_with("panic")
				|| FirstLine.starts_with("thread '")
			{
				"error"
			} else if FirstLine.starts_with("Compiling ")
				|| FirstLine.starts_with("   Compiling ")
				|| FirstLine.contains("Finished")
				|| FirstLine.starts_with("running ")
				|| FirstLine.starts_with("test ")
			{
				"build_output"
			} else if FirstLine.starts_with("error[E")
				|| FirstLine.starts_with("error: ")
				|| FirstLine.starts_with("warning[")
				|| FirstLine.starts_with("warning: ")
				|| (FirstLine.contains("|") && (FirstLine.contains("error") || FirstLine.contains("warning")))
				|| FirstLine.contains("mypy")
				|| FirstLine.contains("clippy")
				|| FirstLine.contains("eslint")
				|| FirstLine.contains("tsc ")
			{
				"linter"
			} else {
				"text"
			}
		},
	}
}
