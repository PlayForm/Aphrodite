//! `[flow]` - the per-turn injected-context budget.

use super::super::super::Struct::Struct;
use crate::State::Struct::AphroditeState;

/// Write the hard cap on all per-turn injected context.
///
/// The cap is kept well below Hermes's 10k-char spill threshold, so the
/// catalog never gets spill-mangled; raising it past that is a real
/// degradation, not a tuning knob.
pub(crate) fn Apply(Configuration:&Struct, State:&mut AphroditeState) {
	// ── Flow-context assembler (05-P1/T5) ──
	// Hard cap for ALL per-turn injected context assembled by
	// `Flow::build_turn_context`; kept well below Hermes's 10k-char spill
	// threshold so the catalog never gets spill-mangled.
	State.Configuration.FlowBudgetChars =
		Configuration.Size("APHRODITE_FLOW_BUDGET_CHARS", "flow", "budget_chars", 4000);
}
