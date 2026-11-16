//! The per-turn context assembler - the single choke point for ALL injected
//! context.
//!
//! Three phases, one file each: `Collect` gathers the candidate sections in
//! survival order, `Budget` drops the optional ones from the bottom and answers a
//! typed outcome, `Log` records what survived. This leaf only sequences them, so
//! the order the model sees is readable in one place instead of being implied by
//! a 130-line body.
//!
//! Section order is not cosmetic. Directives, nudges, the first-turn orientation
//! and the engine-offload prompt are `Required` and survive every budget; the
//! recall catalog is `Optional` and goes first. Getting protection wrong does not
//! merely trim text - it silently removes the directives the session is supposed
//! to be following.

use crate::{
	Flow::{
		Section::Join::Fn::Fn as JoinSections,
		TurnContext::{
			Budget::{Fn::Fn as Trim, Outcome::Enum::Enum as BudgetOutcome},
			Collect::Fn::Fn as Gather,
			Log::Fn::Fn as Trace,
		},
	},
	State::Struct::AphroditeState,
};

/// Assemble the complete per-turn injected context, in fixed order, within
/// `state.flow_budget_chars` BYTES.
///
/// Section order (top = highest survival priority; the drop is bottom-up and only
/// touches `Optional` sections, so the recall catalog goes first and the
/// directives, nudges, first-turn orientation and engine-offload prompt never go
/// at all):
///
/// ```text
/// [aphrodite: first-turn orientation]  (turn 0 only - required)
/// [directives: focus] ...             (build_directive_context - required)
/// [nudge: ...]                        (ephemeral one-shots, at most 2 - required)
/// [engine-offload: ...]               (present only when the engine archived
///                                      this turn - required)
/// [retrieval: ...]                    (opt-in retrieve_guidance line - optional)
/// <poll-worker status>                (optional)
/// [auto-expand: ...]                  (opt-in auto_expand_guidance line - optional)
/// [recall] <catalog summary>          (optional, dropped first)
/// ```
///
/// The limit is measured in bytes, not characters, and the required sections are
/// never dropped: a turn whose required text alone exceeds the budget is returned
/// oversized and reported, because silently truncating the text the session is
/// following would be worse than a large turn.
///
/// `Estimate` is reserved for the P9 telemetry line (chars/4 estimate)
/// and is accepted now so callers don't have to change signature later; T1
/// itself does not render a telemetry line.
pub fn Fn(State:&mut AphroditeState, Estimate:Option<usize>) -> String {
	let _ = Estimate; // reserved for P9/T27 telemetry line
	// DEV-ONLY debug instrumentation (crate::debug): pre-turn timing
	// (captured only when the logger is on - one atomic load when off).
	let T0 = crate::Debug::Level::Enabled::Fn::Fn().then(std::time::Instant::now);

	let Budget = State.Configuration.FlowBudgetChars;

	// Gather every candidate section in fixed order, then drop the optional ones
	// from the bottom until the joined string fits the budget.
	let (Measured, Overflow) = match Trim(Gather(State), Budget) {
		BudgetOutcome::Fits(Measured) => (Measured, None),
		BudgetOutcome::ProtectedOverflow { Surviving, Limit, Excess } => (Surviving, Some((Limit, Excess))),
	};

	let _ = Overflow.map(|(Limit, Excess)| {
		tracing::warn!(
			limit = Limit,
			excess = Excess,
			bytes = Measured.Bytes,
			"turn context exceeds its byte budget: required sections preserved, overflow reported"
		);
	});

	let Joined = JoinSections(&Measured.Sections);
	Trace(State, T0, Budget, Measured.Floor, &Measured.Sections);
	Joined
}
