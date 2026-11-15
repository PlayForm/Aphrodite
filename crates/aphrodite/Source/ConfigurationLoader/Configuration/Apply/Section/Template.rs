//! `[templates]` plus the marker grammar and first-turn session injection.

use super::super::super::Struct::Struct;
use crate::State::Struct::AphroditeState;

/// Write the loaded template model into state and register the marker grammar
/// process-globally.
///
/// The marker builder runs on paths without an `AphroditeState` (hooks, the
/// dylib, the poll worker), so the `[templates.marker]` strings and the
/// `[prompts] ccr_marker_hint` switch must reach it through the global. An
/// absent `[templates]` section RESETS that global to the canonical fallback
/// grammar - a hot-reload that removes the templates must never leave a stale
/// format in force.
///
/// Session injection has a four-step chain, in this order: override → env →
/// `[templates.prompts] session_inject` (canonical) → `[prompts]
/// session_inject` (legacy) → the compiled-in shipped text. An empty string
/// disables the injection entirely, which is why it cannot be treated as
/// "absent".
pub(crate) fn Apply(Configuration:&Struct, State:&mut AphroditeState) {
	// ── [templates] section (pre-Rust port): preview families, marker
	// block, prompt templates, reverse key map ──
	let Templates = Configuration.GetTemplates();
	State.Templates = Templates.clone();

	// ── TOML-driven marker block templates (pre-Rust port) ──
	// Register the `[templates.marker]` format/hint strings and the
	// `[prompts] ccr_marker_hint` knob process-globally (mirroring
	// `SetPreviewMaximumChars`): the marker builder runs on paths without an
	// `AphroditeState` (hooks, dylib, poll worker), so the config must reach
	// it through the global. An absent `[templates]` section resets to the
	// canonical fallback grammar so a hot-reload that removes the templates
	// never leaves a stale format in force.
	let CCRMarkerHint = Configuration.GetBool("APHRODITE_CCR_MARKER_HINT", "prompts", "ccr_marker_hint", false);
	crate::Marker::Format::Const::SetMarkerTemplates(
		Templates.as_ref().and_then(|t| t.Marker.get("format")).map(String::as_str),
		Templates.as_ref().and_then(|t| t.Marker.get("hint")).map(String::as_str),
		CCRMarkerHint,
	);

	// ── First-turn session injection (templates.prompts.session_inject) ──
	// Priority (pre-Rust port): override/env → `[templates.prompts]
	// session_inject` (canonical location) → `[prompts] session_inject`
	// (legacy location) → compiled-in SHIPPED_SESSION_INJECT (so a
	// bare/minimal config still gets a first-turn orientation). Empty string
	// disables it entirely.
	State.SessionInject = match Configuration.Overrides.get("APHRODITE_SESSION_INJECT") {
		Some(v) => v.clone(),
		None => {
			match std::env::var("APHRODITE_SESSION_INJECT") {
				Ok(v) => v,
				Err(_) => {
					match Templates.as_ref().and_then(|t| t.Prompts.get("session_inject")) {
						Some(t) => t.clone(),
						None => {
							Configuration.GetString(
								"APHRODITE_SESSION_INJECT",
								"prompts",
								"session_inject",
								crate::Flow::Prompt::Const::ShippedSessionInject,
							)
						},
					}
				},
			}
		},
	};
}
