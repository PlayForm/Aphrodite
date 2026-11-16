//! `[templates]` plus the first-turn session injection.

use super::super::super::Struct::Struct;
use crate::State::Struct::AphroditeState;

/// Write the loaded template model into state and resolve the session injection.
///
/// This applier writes STATE only. The marker grammar the templates describe is a
/// process global, and it is registered by `Apply::Global::Publish` alongside the
/// preview cap and the template table - one publication point for one document,
/// rather than one here and one there. A section applier that reached for a global
/// was the reason a load could register the marker grammar but not the templates, or
/// vice versa, depending on which caller ran which.
///
/// Session injection has a four-step chain, in this order: override → env →
/// `[templates.prompts] session_inject` (canonical) → `[prompts]
/// session_inject` (legacy) → the compiled-in shipped text. An empty string
/// disables the injection entirely, which is why it cannot be treated as
/// "absent".
pub(crate) fn Apply(Configuration:&Struct, State:&mut AphroditeState) {
	// ── [templates] section (pre-Rust port): preview families, marker
	// block, prompt templates, reverse key map ──
	let Templates = Configuration.Template();
	State.Configuration.Templates = Templates.clone();

	// ── First-turn session injection (templates.prompts.session_inject) ──
	// Priority (pre-Rust port): override/env → `[templates.prompts]
	// session_inject` (canonical location) → `[prompts] session_inject`
	// (legacy location) → compiled-in SHIPPED_SESSION_INJECT (so a
	// bare/minimal config still gets a first-turn orientation). Empty string
	// disables it entirely.
	State.Configuration.SessionInject = match Configuration.Overrides.get("APHRODITE_SESSION_INJECT") {
		Some(v) => v.clone(),
		None => {
			match std::env::var("APHRODITE_SESSION_INJECT") {
				Ok(v) => v,
				Err(_) => {
					match Templates.as_ref().and_then(|t| t.Prompts.get("session_inject")) {
						Some(t) => t.clone(),
						None => {
							Configuration.Text(
								"APHRODITE_SESSION_INJECT",
								"prompts",
								"session_inject",
								crate::Flow::Prompt::Const::Shipped,
							)
						},
					}
				},
			}
		},
	};
}
