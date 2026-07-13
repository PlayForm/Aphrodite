//! Preview builder: content-type detection and human-readable preview strings
//! for compressed CCR content.
//!
//! Used by both the proxy binary and the `aphrodite-hermes` bridge crate
//! (via `crate::preview` re-export).

use headroom_core::transforms;

/// Detect the CCR content-type string for a blob (e.g. `source_code`, `build`,
/// `json_array`). Thin wrapper over the Headroom classifier so downstream
/// crates (aphrodite-hermes) don't need a direct headroom-core dependency.
pub fn detect_type(content: &str) -> String {
	transforms::detect(content).as_str().to_string()
}

/// Build a compact, human-readable preview string for compressed content,
/// shaped per content type (e.g. error/warning counts for build output,
/// +/- line counts for diffs, fn/struct counts for source code) so the LLM
/// gets a useful summary instead of a generic byte/line count wherever a
/// richer signal is available.
pub fn build_preview(type_str: &str, content: &str) -> String {
	let lines = content.lines().count();
	let bytes = content.len();
	match type_str {
		"build" | "build_output" | "build_error" => {
			let e = content.matches("error").count();
			let w = content.matches("warning").count();
			format!("[{}:{}E {}W {}L]", type_str, e, w, lines)
		},
		"diff" => {
			let f = content.matches("diff --git").count();
			let a = content.lines().filter(|l| l.starts_with('+') && !l.starts_with("+++")).count();
			let d = content.lines().filter(|l| l.starts_with('-') && !l.starts_with("---")).count();
			format!("[diff:{}F +{}/-{} {}L]", f, a, d, lines)
		},
		"source_code" | "code_rust" | "code_python" | "code_go" | "code_js" | "code_ts" => {
			// Enrich with the structure map (fns/structs/traits/impls/classes/types
			// + first signature) so the dylib/hook path matches the proxy's preview
			// quality, instead of a bare substring count.
			let st = crate::struct_extract::extract_code_structure(content, "");
			let mut parts: Vec<String> = Vec::new();
			for (key, label) in [
				("fns", "fns"),
				("structs", "structs"),
				("traits", "traits"),
				("impls", "impls"),
				("classes", "classes"),
				("types", "types"),
			] {
				if let Some(v) = st.get(key) {
					if !v.is_empty() {
						parts.push(format!("{}{}", v.len(), label));
					}
				}
			}
			let summary = if parts.is_empty() {
				format!("{}fns", content.matches("fn ").count() + content.matches("def ").count())
			} else {
				parts.join("|")
			};
			let sig = st
				.get("fns")
				.and_then(|v| v.first())
				.map(|s| format!(" {}", s.chars().take(48).collect::<String>().trim()))
				.unwrap_or_default();
			format!("[code:{}{} {}L]", summary, sig, lines)
		},
		"search" => {
			let h = content.lines().filter(|l| l.contains(':')).count();
			format!("[search:{}hits {}L]", h, lines)
		},
		"json_array" => {
			let i = content.matches("{\"").count();
			format!("[json:{}items {}L]", i, lines)
		},
		// `hooks::transform_terminal_output` overrides the classified type to
		// "terminal" when the content looks like a shell/exit-code trace, but
		// this function had no matching arm for it, so the preview silently
		// fell through to the generic `_` branch (F10) - a bare line/byte
		// count with no exit-code or last-output-line context, the exact
		// signal a terminal preview exists to surface.
		"terminal" => {
			let exit_line = content
				.lines()
				.rev()
				.find(|l| l.contains("exit code:") || l.contains("Error:"))
				.map(|l| l.trim());
			let last_line = content.lines().rev().find(|l| !l.trim().is_empty()).map(|l| l.trim());
			let summary = exit_line.or(last_line).unwrap_or("").chars().take(60).collect::<String>();
			format!("[terminal:{}L {}]", lines, summary)
		},
		_ => format!("[{}:{}L {}B]", type_str, lines, bytes),
	}
}
