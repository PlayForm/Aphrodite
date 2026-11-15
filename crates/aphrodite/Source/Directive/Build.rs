//! Context-string construction for `PreLLMCall` injection.

use std::collections::HashMap;

use super::Model::{MaximumCombinedChars, Struct};

/// Build the context string injected into `PreLLMCall`.
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
/// full (per-file `MaximumDirectiveChars`-capped) body, stripped of leading `#`
/// markers, under a combined-output cap so several active directives can't
/// blow past the context budget this feature is supposed to respect.
///
/// T19: the cap is compared as a CHAR count, not a byte count. `out.len()`
/// would trip the truncation on multibyte content (em-dashes etc.) that never
/// exceeded the char budget, silently cutting directive text that fit.
pub fn Fn(all:&HashMap<String, Struct>, Active:&[String]) -> String {
	match Active.is_empty() {
		true => return String::new(),
		false => (),
	}

	let Names:Vec<&str> = Active.iter().map(|s| s.as_str()).collect();
	let mut Out = format!("[directives: {}]\n", Names.join(", "));

	Active.iter().filter_map(|name| all.get(name)).for_each(|d| {
		Out.push_str(&format!("{}:\n", d.name));

		d.Content.lines().for_each(|Line| {
			let Line = Line.trim_start_matches('#').trim();

			match Line.is_empty() {
				true => (),
				false => {
					Out.push_str("  ");
					Out.push_str(Line);
					Out.push('\n');
				},
			}
		});
	});

	// T19: the gate must compare CHAR count against MAX_COMBINED_CHARS -
	// `out.len()` is a byte length, so multibyte content (em-dashes etc.)
	// tripped the truncation even when it fit the char cap, silently
	// cutting directive text that never exceeded the budget.
	match Out.chars().count() > MaximumCombinedChars {
		true => {
			let Trunc:String = Out.chars().take(MaximumCombinedChars).collect();

			Out = format!("{}...\n", Trunc);
		},
		false => (),
	}

	Out
}
