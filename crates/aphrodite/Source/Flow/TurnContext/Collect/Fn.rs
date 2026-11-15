//! Gather every section a turn may inject, in fixed order.
//!
//! The order here IS the survival order: the budget drop removes from the end
//! of this list, so a section pushed earlier is a section that survives longer.
//! Two constraints are invisible in the code but load-bearing - the
//! engine-offload trigger runs BEFORE the catalog summary (the engine mutates
//! the conversation index, and the catalog reads it), and the always-survive
//! count is computed after both (it reads the nudge list the catalog does not
//! touch, but keeping one evaluation point avoids two answers to one
//! question).

use crate::{
	Flow::{
		AutoExpandGuidanceBlock,
		BuildFirstTurnInjection,
		EngineOffloadBlock,
		NudgesPresent,
		RenderNudges,
		RetrievalGuidanceBlock,
	},
	State::AphroditeState,
};

/// Every candidate section for this turn, in survival order, plus the number
/// of leading entries the budget may never drop.
///
/// The returned count is an upper bound: the caller clamps it to the number of
/// sections actually present, so an empty turn cannot underflow its own floor.
/// It counts only PRESENT always-survive sections - a section that rendered
/// nothing is not a section the drop loop has to protect.
pub(crate) fn CollectSections(State:&mut AphroditeState) -> (Vec<String>, usize) {
	// ── First-turn session injection (always-survive, top priority) ──
	let SessionInject = BuildFirstTurnInjection(State);

	// ── Always-survive sections (top priority) ──
	let Directives = crate::Directive::Build::Fn(&State.Directives, &State.ActiveDirectives);
	let Nudges = RenderNudges(State);

	// ── Context-engine trigger + offload prompt (fires only when the engine
	// archived messages this turn; runs before the recall summary so any
	// engine state mutation lands before the catalog reads the markers) ──
	let EngineOffload = EngineOffloadBlock(State);
	let EngineOffloadPresent = !EngineOffload.is_empty();

	// ── Retrieval guidance (opt-in via [prompts] retrieve_guidance; drops
	// before bg status / recall under budget pressure) ──
	let Retrieve = RetrievalGuidanceBlock(State);

	// ── Auto-expand guidance (opt-in via state.auto_expand; drops before
	// bg status / recall under budget pressure) ──
	let AutoExpand = AutoExpandGuidanceBlock(State);

	// ── Droppable section: recall catalog, delta-only (04-F1) ──
	let Recall = crate::Session::CatalogSummary(State);

	// ── Poll-worker status (above recall, dropped after recall) ──
	let BgStatus = if State.PollWorkerEnabled {
		crate::PollWorker::Status::Fn::RenderBgTaskStatus(State)
	} else {
		String::new()
	};

	// Assemble top-down, then drop from the bottom until within budget.
	let mut Sections:Vec<String> = Vec::new();
	if !SessionInject.is_empty() {
		Sections.push(format!("[aphrodite: first-turn orientation]\n{}", SessionInject.trim_end()));
	}
	if !Directives.is_empty() {
		Sections.push(Directives.trim_end().to_string());
	}
	if !Nudges.is_empty() {
		Sections.push(Nudges);
	}
	if EngineOffloadPresent {
		Sections.push(EngineOffload);
	}
	if !Retrieve.is_empty() {
		Sections.push(Retrieve);
	}
	if !BgStatus.is_empty() {
		Sections.push(BgStatus);
	}
	if !AutoExpand.is_empty() {
		Sections.push(AutoExpand);
	}
	// The recall block - RETRIEVE_HINT removed per 04-F2 (tool schemas
	// in the system prompt already teach retrieval; repeating it every
	// turn was 56 chars of pure noise, ~2.8k chars over 50 turns).
	if !Recall.is_empty() {
		Sections.push(format!("[recall]\n{}\n", Recall.trim_end()));
	}
	// Count only PRESENT always-survive sections (fix 11: the old
	// `is_empty() as usize` casts were inverted - they counted ABSENT
	// sections, so a turn with an inject + directive + nudge present could
	// compute always_survive == 1 and drop directives/nudges under budget
	// pressure). Session inject non-empty, directives non-empty, nudges
	// present, engine-offload present; keep the min with the section count
	// so an empty turn never underflows the floor.
	let AlwaysSurvive = (!SessionInject.is_empty()) as usize
		+ (!Directives.is_empty()) as usize
		+ NudgesPresent(State) as usize
		+ EngineOffloadPresent as usize;

	(Sections, AlwaysSurvive)
}
