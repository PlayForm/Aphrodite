//! Gather every section a turn may inject, in fixed order.
//!
//! The order here IS the survival order: the budget drops from the end of this
//! list, so a section placed earlier survives longer - and each section carries
//! its own protection, so the floor is not a number this function has to keep in
//! step with the list. Two constraints are invisible in the code but load-bearing
//! - the engine-offload trigger runs BEFORE the catalog summary (the engine
//!   mutates the conversation index, and the catalog reads it), and the sections are
//!   all rendered before the list is built, so no rendered section is dropped by a
//!   later mutation.
//!
//! The list is built as a filter over eight candidates rather than as eight
//! "present, therefore push" branches: presence is a predicate on the rendered
//! section, so it belongs in the candidate expression and not in a statement that
//! mutates a vector. The order of the candidates is the survival order, and it is
//! the only thing about this function a reader has to get right.

use crate::{
	Flow::{
		Engine::Offload::Block::Fn::Fn as Offload,
		FirstTurn::Fn::Fn as FirstTurn,
		Nudge::Render::Fn::Fn as Nudge,
		Retrieval::{AutoExpand::Fn::Fn as Expand, Guidance::Fn::Fn as Guidance},
		Section::Struct::Struct as Section,
	},
	State::Struct::AphroditeState,
};

/// Every candidate section for this turn, in survival order.
///
/// Each candidate is a [`Section`] carrying its own protection: the first-turn
/// orientation, the directives, the nudges and the engine-offload prompt are
/// `Required` (the session is following that text), and the guidance lines, the
/// poll-worker status and the recall catalog are `Optional` (dropped from the
/// bottom when the turn is too big). A section that rendered nothing is not a
/// candidate at all, so the budget never protects text that does not exist.
pub(crate) fn Fn(State:&mut AphroditeState) -> Vec<Section> {
	// ── First-turn session injection (always-survive, top priority) ──
	let SessionInject = FirstTurn(State);

	// ── Always-survive sections (top priority) ──
	let Directives = crate::Directive::Build::Fn(&State.Directive.Loaded, &State.Directive.Active);
	let Nudges = Nudge(State);

	// ── Context-engine trigger + offload prompt (fires only when the engine
	// archived messages this turn; runs before the recall summary so any engine
	// state mutation lands before the catalog reads the markers) ──
	let EngineOffload = Offload(State);

	// ── Retrieval guidance (opt-in via [prompts] retrieve_guidance; drops before
	// bg status / recall under budget pressure) ──
	let Retrieve = Guidance(State);

	// ── Auto-expand guidance (opt-in via state.auto_expand; drops before bg status
	// / recall under budget pressure) ──
	let AutoExpand = Expand(State);

	// ── Droppable section: recall catalog, delta-only (04-F1) ──
	let Recall = crate::Session::Catalog::Fn::Fn(State);

	// ── Poll-worker status (above recall, dropped after recall). The renderer owns
	// the feature flag, so a disabled poll worker is an empty section here. ──
	let BgStatus = crate::PollWorker::Status::Fn::Fn(State);

	[
		(!SessionInject.is_empty())
			.then(|| Section::Required(format!("[aphrodite: first-turn orientation]\n{}", SessionInject.trim_end()))),
		(!Directives.is_empty()).then(|| Section::Required(Directives.trim_end().to_string())),
		(!Nudges.is_empty()).then(|| Section::Required(Nudges)),
		(!EngineOffload.is_empty()).then(|| Section::Required(EngineOffload)),
		(!Retrieve.is_empty()).then(|| Section::Optional(Retrieve)),
		(!BgStatus.is_empty()).then(|| Section::Optional(BgStatus)),
		(!AutoExpand.is_empty()).then(|| Section::Optional(AutoExpand)),
		// The recall block - RETRIEVE_HINT removed per 04-F2 (tool schemas in the
		// system prompt already teach retrieval; repeating it every turn was 56
		// chars of pure noise, ~2.8k chars over 50 turns).
		(!Recall.is_empty()).then(|| Section::Optional(format!("[recall]\n{}\n", Recall.trim_end()))),
	]
	.into_iter()
	.flatten()
	.collect()
}
