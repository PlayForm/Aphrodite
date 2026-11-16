//! The opt-in `[prompts] retrieve_guidance` line.
//!
//! Teaches retrieval VOCABULARY - what a marker is, when resolving one is
//! worth its bytes - and nothing about the machinery that produced it. The
//! compression pipeline stays invisible to the model: a line that explained
//! the splitter would teach it to reason about the tool instead of its task.

use super::{Standard::Const::Text as Standard, Verbose::Const::Text as Verbose};
use crate::State::Struct::AphroditeState;

/// Retrieval-guidance section (prompt knob `[prompts] retrieve_guidance`):
/// minimal (default) = current behavior, nothing extra; standard = one line;
/// verbose = extended. An unrecognized value warns and degrades to standard
/// rather than injecting nothing (warn + degrade house style; never panics).
/// Empty string = no section this turn.
pub(crate) fn Fn(State:&AphroditeState) -> String {
	let Text = match State.RetrieveGuidance.as_str() {
		"minimal" | "" => return String::new(),
		"verbose" => Verbose,
		Other => {
			tracing::warn!(Value = %Other, "unknown retrieve_guidance value; degrading to standard");
			Standard
		},
	};
	format!("[retrieval: {}]", Text)
}
