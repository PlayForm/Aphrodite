//! Runtime directive actions against live state.

use crate::State::Struct::AphroditeState;
use super::{
	Refusal,
	Verb::{Add, List, Load, Remove, Reset, Swap},
};

/// Handle a directive action against live state.
///
/// This is the single implementation of the action vocabulary. Both `AphroditeDirective` (the
/// extern fn) and `AphroditeDispatch`'s `"directive"` arm delegate here (01-F8) - previously ~40
/// lines of this logic were duplicated between the two, with divergent error shapes (the extern
/// fn returned an error via a separate top-level `ToJSONError` call, the dispatch arm embedded
/// `{"error": ...}` inside an otherwise-success value). This always returns the latter shape, so
/// a caller can pass it straight through its own success serializer.
///
/// The action strings are a wire contract: they are what `aphrodite_directive` accepts and what
/// the unknown-action message advertises, so they are matched literally and the refusal lists
/// them - from [`Refusal::Fn::Fn`], which is also where the unknown-NAME refusal lives, so the
/// vocabulary's two refusals are one table rather than two spellings.
///
/// Every successful mutation latches `ManualDirectiveTurn` (P6) - inside the leaf that performs
/// it, because a manual choice must switch phase-aware auto-swaps off and a *failed* action must
/// not.
pub fn Fn(State:&mut AphroditeState, Verb:&str, name:&str) -> serde_json::Value {
	match Verb {
		"list" => List::Fn::Fn(State),
		"swap" => Swap::Fn::Fn(State, name),
		"add" => Add::Fn::Fn(State, name),
		"load" => Load::Fn::Fn(State, name),
		"remove" => Remove::Fn::Fn(State, name),
		"reset" => Reset::Fn::Fn(State),
		_ => Refusal::Fn::Fn(&Refusal::Enum::Enum::Action, Verb),
	}
}
