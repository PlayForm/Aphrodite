//! Drop ephemeral directives whose turn lifetime has elapsed.
//!
//! Called from `PostLLMCall` AFTER the turn counter advances, which is what
//! makes `ttl_turns = 1` mean "render once": a nudge pushed during turn N
//! expires at N + ttl, so it is still live when turn N + 1 renders and dead by
//! the time turn N + 2 does. Purging before the counter advanced would delete
//! the nudge before the model ever saw it.

use crate::State::Struct::AphroditeState;

/// Purge expired ephemeral directives (P3/T9). Called from `PostLLMCall`
/// AFTER `NextTurn` advances the counter, so a nudge pushed during turn N
/// (expiring at N + ttl) renders in turn N+1's context exactly once for ttl=1.
pub fn Fn(State:&mut AphroditeState) {
	let Counter = State.TurnCounter;
	State
		.EphemeralDirectives
		.retain(|e| e.ExpiresAfterTurn.is_none_or(|Exp| Exp >= Counter));
}
