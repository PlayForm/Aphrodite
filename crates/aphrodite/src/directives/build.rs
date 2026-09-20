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
	// T19: the gate must compare CHAR count against MAX_COMBINED_CHARS -
	// `out.len()` is a byte length, so multibyte content (em-dashes etc.)
	// tripped the truncation even when it fit the char cap, silently
	// cutting directive text that never exceeded the budget.
	if out.chars().count() > MAX_COMBINED_CHARS {
		let trunc:String = out.chars().take(MAX_COMBINED_CHARS).collect();
		out = format!("{}…\n", trunc);
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	fn active(name:&str, content:&str) -> (HashMap<String, Directive>, Vec<String>) {
		let mut all = HashMap::new();
		all.insert(
			name.to_string(),
			Directive { name:name.to_string(), content:content.to_string() },
		);
		let active = vec![name.to_string()];
		(all, active)
	}

	// ── T19: the combined-output gate must be CHAR-based, not byte-based.
	// Em-dashes are 3 bytes each, so content that fits the 4000-char cap
	// still exceeds it in bytes - the old `out.len() > MAX_COMBINED_CHARS`
	// check truncated it anyway. The result must stay within the char cap.
	#[test]
	fn test_combined_char_cap_counts_chars_not_bytes() {
		let (all, active) = active("focus", &"\u{2014}".repeat(3000));
		let out = build_directive_context(&all, &active);
		// Char count (prefix + 3000 dashes + newline) is within the cap...
		assert!(
			out.chars().count() <= MAX_COMBINED_CHARS,
			"char-count output must stay within the char cap"
		);
		// ...but the byte length exceeds it, which would have tripped the
		// old byte-based gate and truncated valid content.
		assert!(out.len() > MAX_COMBINED_CHARS, "byte length exceeds the cap");
		// Untruncated: full content present, no ellipsis.
		assert!(!out.contains('…'), "within-cap content must not be truncated");
		assert!(out.ends_with("\u{2014}\n"));
	}

	// ── T19: genuinely over-cap content is still truncated, char-exact:
	// exactly MAX_COMBINED_CHARS chars of content survive before the
	// ellipsis newline.
	#[test]
	fn test_combined_char_cap_truncates_over_cap_content() {
		let (all, active) = active("focus", &"-".repeat(4100));
		let out = build_directive_context(&all, &active);
		assert!(out.ends_with("…\n"), "over-cap content must be truncated");
		let content_part = out.trim_end_matches("…\n");
		assert_eq!(content_part.chars().count(), MAX_COMBINED_CHARS);
		assert_eq!(out.chars().count(), MAX_COMBINED_CHARS + 2);
	}
}
