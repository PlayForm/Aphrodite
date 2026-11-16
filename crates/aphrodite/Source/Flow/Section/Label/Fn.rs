//! The short label of an injected section, derived from its first line.
//!
//! DEV-ONLY: this feeds the `pre_turn_sections` debug line, never the model. The
//! section order it labels is fixed by `TurnContext::Fn`.

use crate::Flow::Section::Label::Table::Const::Prefixes;

/// The label for a section, or `"other"` when no prefix matches.
pub(crate) fn Fn(Text:&str) -> &'static str {
	match Prefixes.iter().find(|(Prefix, _)| Text.starts_with(Prefix)) {
		Some((_, Label)) => Label,
		None => "other",
	}
}
