//! Context-string construction for `PreLLMCall` injection.

use std::collections::HashMap;

use super::{Cap::Combined::Const::Const as Maximum, Model::Struct};

/// Build the context string injected into `PreLLMCall`.
///
/// ```text
/// [directives: focus]
/// focus:
///   focus - stay targeted, minimal tool usage
///   Each turn: use at most 1-2 tools. Prefer retrieval over re-reading.
///   - One primary action per turn
///   - ...
/// ```
///
/// Returns the empty string when no directive is active - not a header with no bodies under it,
/// which would spend context saying nothing.
///
/// 01-F5: this used to inject only each directive's first line (a markdown title, e.g.
/// `# focus - stay targeted, minimal tool usage`), so the bullets carrying the actual behavioural
/// instructions never travelled. It now injects the full (per-file-capped) body, stripped of
/// leading `#` markers, under a combined-output cap so several active directives cannot blow past
/// the context budget this feature exists to respect.
///
/// The active list is the ORDER: directives render in the order they were activated, and an
/// active name with no loaded directive contributes nothing rather than a placeholder header.
///
/// The heading is the map KEY, not the entry's own `name` copy. They are the same string by
/// construction - the loader and the built-in set build one from the other - but a `HashMap` can
/// be edited into a state where they disagree, and a directive rendered under another directive's
/// heading would be injected as the wrong instruction. The key is what `aphrodite_directive`
/// addresses, so it is what the model must see.
pub fn Fn(all:&HashMap<String, Struct>, Active:&[String]) -> String {
	// `then` rather than a pre-built vector: with no active directive there is no name list to
	// build and no header to render.
	let Some(Names) = (!Active.is_empty()).then(|| Active.iter().map(String::as_str).collect::<Vec<_>>()) else {
		return String::new();
	};

	let mut Out = format!("[directives: {}]\n", Names.join(", "));

	Active
		.iter()
		.filter_map(|name| all.get(name).map(|Directive| (name, Directive)))
		.for_each(|(Name, Directive)| Out.push_str(&Section(Name, Directive)));

	// T19: the cap is compared as a CHARACTER count. `Out.len()` is a byte length, so multibyte
	// content (em-dashes etc.) tripped the truncation on text that never exceeded the char
	// budget, silently cutting directives that fit.
	match Out.char_indices().nth(Maximum) {
		Some((Offset, _Char)) => format!("{}...\n", &Out[..Offset]),
		None => Out,
	}
}

/// One directive's section: its heading, then every non-blank body line indented.
///
/// A blank line contributes nothing - the `filter` is the gate, and the accepted
/// path is the only one that allocates a rendered line.
fn Section(Name:&str, Directive:&Struct) -> String {
	let Body:String = Directive
		.Content
		.lines()
		.map(|Line| Line.trim_start_matches('#').trim())
		.filter(|Line| !Line.is_empty())
		.map(|Line| format!("  {Line}\n"))
		.collect();

	format!("{Name}:\n{Body}")
}
