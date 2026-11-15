//! Queue an inline nudge with a turn lifetime.
//!
//! The nudge renders on the next `PreLLMCall` and self-purges after
//! `TtlTurns`, so `ttl_turns = 1` is a one-shot: rendered once, then gone.
//! The store holds at most four entries and evicts the oldest, so a caller
//! that pushes every turn cannot accumulate unbounded text into the budget.

use crate::State::Directive::Struct::ActiveDirective;
use crate::State::Struct::AphroditeState;

/// Push an inline nudge that renders on the next `PreLLMCall` and self-purges
/// after `TtlTurns` (P3/T9). `ttl_turns = 1` is a one-shot: rendered on the
/// next turn, purged at the following `PostLLMCall`. At most 4 ephemeral
/// entries are stored; pushing a 5th drops the oldest.
pub fn PushNudge(State:&mut AphroditeState, Text:&str, TtlTurns:usize) {
	let Expires = Some(State.TurnCounter + TtlTurns);
	State.EphemeralDirectives.push(ActiveDirective {
		name:String::new(),
		inline:Some(Text.to_string()),
		ExpiresAfterTurn:Expires,
	});
	while State.EphemeralDirectives.len() > 4 {
		// Evict the oldest (front). `drain(..1)` was chosen over a VecDeque
		// type change to keep the field type stable for consumers (fix 12).
		State.EphemeralDirectives.drain(..1);
	}
}
