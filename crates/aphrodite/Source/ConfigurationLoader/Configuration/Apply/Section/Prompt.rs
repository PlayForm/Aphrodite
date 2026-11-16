//! `[prompts]` - how much the model is told about retrieval, and how the
//! recall catalog is annotated.

use super::super::super::Struct::Struct;
use crate::State::Struct::AphroditeState;

/// Write the two prompt knobs that steer per-turn teaching.
///
/// Both default to the quietest behaviour (`minimal`, hints off), so a config
/// that predates these keys behaves exactly as it did before they existed.
pub(crate) fn Apply(Configuration:&Struct, State:&mut AphroditeState) {
	// ── Prompt knobs (pre-Rust port; previously parsed with no consumer) ──
	// `RetrieveGuidance` (minimal|standard|verbose): how much CCR retrieval
	// explanation `Flow::build_turn_context` injects per turn. Default
	// minimal = current behavior.
	State.RetrieveGuidance =
		Configuration.Text("APHRODITE_RETRIEVE_GUIDANCE", "prompts", "retrieve_guidance", "minimal");
	// `CatalogIntentHints`: show `intent=read` alongside the recent CCR
	// hashes in the recall catalog output (`Session::catalog_summary`).
	// Default off = current behavior.
	State.CatalogIntentHints =
		Configuration.Bool("APHRODITE_CATALOG_INTENT_HINTS", "prompts", "catalog_intent_hints", false);
}
