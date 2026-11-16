//! Render the active inline nudges as `[nudge: <text>]` lines.
//!
//! At most two render per turn, newest wins, and an entry renders only while
//! it has not expired. The two-line cap is the whole point of nudges: they are
//! one-shots that correct the model's current behaviour, not a second
//! directive channel, and an uncapped list would grow into one.

use crate::State::Struct::AphroditeState;

/// Render at most 2 active inline nudges as `[nudge: <text>]` lines, newest
/// wins (P3/T9). An entry renders while it has not expired
/// (`expires_after_turn >= turn_counter`, or `None` = permanent).
pub fn Fn(State:&AphroditeState) -> String {
	let mut Lines:Vec<String> = State
		.Directive
		.Ephemeral
		.iter()
		.rev() // newest first
		.filter(|e| e.inline.is_some())
		.filter(|e| e.ExpiresAfterTurn.is_none_or(|Exp| Exp >= State.Turn.Counter))
		.take(2)
		.filter_map(|e| e.inline.as_ref().map(|t| format!("[nudge: {t}]")))
		.collect();
	// Preserve newest-last visual order (we collected newest-first).
	Lines.reverse();
	Lines.join("\n")
}
