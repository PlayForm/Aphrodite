//! The per-turn context assembler - the single choke point for ALL injected
//! context.
//!
//! Three phases, one file each: `Collect` gathers the candidate sections in
//! survival order, `Budget` drops from the bottom until the joined string
//! fits, `Log` records what survived. This leaf only sequences them, so the
//! order the model sees is readable in one place instead of being implied by
//! a 130-line body.
//!
//! Section order is not cosmetic. Directives, nudges, the first-turn
//! orientation and the engine-offload prompt survive every budget; the recall
//! catalog is dropped first. Getting the floor wrong does not merely trim
//! text - it silently removes the directives the session is supposed to be
//! following.

use crate::{
	Flow::{
		JoinSections,
		TurnContext::{Budget::DropToBudget, Collect::CollectSections, Log::LogPreTurn},
	},
	State::AphroditeState,
};

/// Assemble the complete per-turn injected context, in fixed order, hard-capped
/// at `state.flow_budget_chars`.
///
/// Section order (top = highest survival priority; drop order is bottom-up:
/// the recall catalog goes first under budget pressure, directives + nudges
/// + the engine-offload prompt never drop):
///
/// ```text
/// [directives: focus] ...        (build_directive_context - never dropped)
/// [nudge: ...]                   (ephemeral one-shots, at most 2 - never dropped)
/// [engine-offload: ...]          (context-engine archive prompt - present only
///                                 when the engine archived this turn; never dropped)
/// [retrieval: ...]               (opt-in retrieve_guidance line - droppable)
/// [auto-expand: ...]             (opt-in auto_expand_guidance line - droppable)
/// [recall] <catalog summary>     (dropped first under budget pressure)
/// ```
///
/// `EstRequestBytes` is reserved for the P9 telemetry line (chars/4 estimate)
/// and is accepted now so callers don't have to change signature later; T1
/// itself does not render a telemetry line.
pub fn BuildTurnContext(State:&mut AphroditeState, EstRequestBytes:Option<usize>) -> String {
	let _ = EstRequestBytes; // reserved for P9/T27 telemetry line
	// DEV-ONLY debug instrumentation (crate::debug): pre-turn timing
	// (captured only when the logger is on - one atomic load when off).
	let T0 = crate::Debug::Level::Enabled::Fn::Fn().then(std::time::Instant::now);

	let Budget = State.FlowBudgetChars;

	// Gather every candidate section in fixed order, then drop from the bottom
	// until the joined string fits the budget.
	let (Sections, AlwaysSurvive) = CollectSections(State);
	let (Sections, AlwaysSurvive) = DropToBudget(Sections, AlwaysSurvive, Budget);

	let Joined = JoinSections(&Sections);
	LogPreTurn(State, T0, Budget, AlwaysSurvive, &Sections);
	Joined
}
