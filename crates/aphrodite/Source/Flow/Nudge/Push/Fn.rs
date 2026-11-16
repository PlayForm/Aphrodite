//! Queue an inline nudge with a turn lifetime.
//!
//! The nudge renders on the next `PreLLMCall` and self-purges after `TtlTurns`, so
//! `ttl_turns = 1` is a one-shot: rendered once, then gone. The store holds at most
//! `Push::Const::Maximum` entries and drops the oldest, so a caller that pushes every turn
//! cannot accumulate unbounded text into the budget.
//!
//! One push can only overflow the bound by one, so the eviction is a single drain
//! of the excess rather than a repair loop: `drain(..0)` is a no-op, which is what
//! makes the in-bounds case cost nothing to express.

use crate::{
	Flow::Nudge::Push::Const::Maximum,
	State::{Directive::Entry::Struct::Struct as ActiveDirective, Struct::AphroditeState},
};

/// Push an inline nudge that renders on the next `PreLLMCall` and self-purges after
/// `TtlTurns` (P3/T9). `ttl_turns = 1` is a one-shot: rendered on the next turn,
/// purged at the following `PostLLMCall`. At most four ephemeral entries are
/// stored; pushing a 5th drops the oldest.
pub fn Fn(State:&mut AphroditeState, Text:&str, TtlTurns:usize) {
	let Expires = Some(State.Turn.Counter + TtlTurns);
	State.Directive.Ephemeral.push(ActiveDirective {
		name:String::new(),
		inline:Some(Text.to_string()),
		ExpiresAfterTurn:Expires,
	});

	let Overflow = State.Directive.Ephemeral.len().saturating_sub(Maximum);

	State.Directive.Ephemeral.drain(..Overflow);
}
