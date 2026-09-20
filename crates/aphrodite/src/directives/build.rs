//! Context-string construction for `pre_llm_call` injection.

use std::collections::HashMap;

use super::model::{Directive, MAX_COMBINED_CHARS};

/// Build the context string injected into `pre_llm_call`.
/// Format:
/// ```text
/// [directives: focus]
/// focus:
///   focus - stay targeted, minimal tool usage
///   Each turn: use at most 1-2 tools. Prefer retrieval over re-reading.
///   - One primary action per turn
///   - ...
/// ```
/// Returns empty string if no directives are active.
///
/// 01-F5: previously injected only each directive's first line (a markdown
/// title, e.g. `# focus - stay targeted, minimal tool usage`) - the bullets
/// with the actual behavioral instructions never traveled. Now injects the
/// full (per-file `MAX_DIRECTIVE_CHARS`-capped) body, stripped of leading `#`
/// markers, under a combined-output cap so several active directives can't
/// blow past the context budget this feature is supposed to respect.
pub fn build_directive_context(all:&HashMap<String, Directive>, active:&[String]) -> String {
	if active.is_empty() {
		return String::new();
	}
	let names:Vec<&str> = active.iter().map(|s| s.as_str()).collect();
	let mut out = format!("[directives: {}]\n", names.join(", "));
	for name in active {
		if let Some(d) = all.get(name) {
			out.push_str(&format!("{}:\n", d.name));
			for line in d.content.lines() {
				let line = line.trim_start_matches('#').trim();
				if !line.is_empty() {
					out.push_str("  ");
					out.push_str(line);
					out.push('\n');
				}
			}
		}
	}
	if out.len() > MAX_COMBINED_CHARS {
		let trunc:String = out.chars().take(MAX_COMBINED_CHARS).collect();
		out = format!("{}…\n", trunc);
	}
	out
}
