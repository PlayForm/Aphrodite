//! The opt-in `[prompts] retrieve_guidance` line.
//!
//! Teaches retrieval VOCABULARY - what a marker is, when resolving one is
//! worth its bytes - and nothing about the machinery that produced it. The
//! compression pipeline stays invisible to the model: a line that explained
//! the splitter would teach it to reason about the tool instead of its task.

use crate::State::Struct::AphroditeState;

/// Retrieval-guidance text for `retrieve_guidance = "standard"`.
const RetrievalGuidanceStandard:&str = "CCR markers compress large tool output. Use aphrodite_retrieve(hash) when the \
                                        preview doesn't tell you enough; batch pending markers into one turn.";

/// Retrieval-guidance text for `retrieve_guidance = "verbose"`.
const RetrievalGuidanceVerbose:&str = "CCR markers compress large tool output. Use aphrodite_retrieve(hash) when the \
                                       preview doesn't tell you enough; batch pending markers into one turn. Prefer \
                                       granular retrieval - pull the exact piece you need. Never re-read a file you \
                                       hold a marker for; never invent content you did not retrieve.";

/// Retrieval-guidance section (prompt knob `[prompts] retrieve_guidance`):
/// minimal (default) = current behavior, nothing extra; standard = one line;
/// verbose = extended. An unrecognized value warns and degrades to standard
/// rather than injecting nothing (warn + degrade house style; never panics).
pub(crate) fn RetrievalGuidanceBlock(State:&AphroditeState) -> String {
	let Text = match State.RetrieveGuidance.as_str() {
		"minimal" | "" => return String::new(),
		"verbose" => RetrievalGuidanceVerbose,
		Other => {
			tracing::warn!(Value = %Other, "unknown retrieve_guidance value; degrading to standard");
			RetrievalGuidanceStandard
		},
	};
	format!("[retrieval: {}]", Text)
}
