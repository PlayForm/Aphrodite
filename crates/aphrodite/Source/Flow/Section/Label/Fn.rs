//! Section bookkeeping for the assembler: the debug label for one section,
//! the newline join that concatenates them, and the presence test that sets
//! the never-drop floor.
//!
//! `JoinSections` is the only one that touches rendered text, and it exists
//! so the budget check measures exactly the string the model will receive -
//! measuring any other concatenation would let a turn exceed its budget while
//! the check said it fit.

use crate::State::AphroditeState;

/// DEV-ONLY: short label for an injected section, derived from its first
/// line (the section order is fixed by [`BuildTurnContext`]).
pub(crate) fn SectionLabel(s:&str) -> &'static str {
	if s.starts_with("[directives") {
		"directives"
	} else if s.starts_with("[nudge") {
		"nudges"
	} else if s.starts_with("[engine-offload") {
		"engine_offload"
	} else if s.starts_with("[retrieval") {
		"retrieval"
	} else if s.starts_with("[auto-expand") {
		"auto_expand"
	} else if s.starts_with("[recall]") {
		"recall"
	} else if s.starts_with("[aphrodite: first-turn") {
		"session_inject"
	} else {
		"other"
	}
}

/// Number of leading "never-drop" sections currently present (directives +
/// nudge block). Used to compute the floor for bottom-up dropping.
pub(crate) fn NudgesPresent(State:&AphroditeState) -> bool {
	State
		.EphemeralDirectives
		.iter()
		.any(|e| e.inline.is_some() && e.ExpiresAfterTurn.is_none_or(|Exp| Exp >= State.TurnCounter))
}

/// Join the surviving sections with a single newline between each.
pub(crate) fn JoinSections(Sections:&[String]) -> String { Sections.join("\n") }
