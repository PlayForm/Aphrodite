//! The `PreLLMCall` nudge entry point: plan, then apply.
//!
//! Bounded on purpose. Every nudge spends part of the per-turn directive budget,
//! so the pass emits at most two (newest-first) and a third running build cannot
//! crowd out the behavioural directives the turn needs.
//!
//! The plans are collected before anything is pushed, which is what makes the
//! selection a query over the task set instead of a mutation of the directive
//! queue interleaved with it: the tasks are read under one borrow, the pushes
//! happen under another.

use crate::{
	Flow::Nudge::Push::Fn::Fn as PushNudge,
	PollWorker::{
		Check::{Plan::Struct::Struct as Nudge, Render::Fn::Fn as Render, Select::Fn::Fn as Select},
		Registry::ReadFrom::Fn::Fn as ReadFrom,
	},
	State::Struct::AphroditeState,
};

/// Check all background tasks and push ephemeral nudges for any that need the
/// agent's attention.
pub fn Fn(State:&mut AphroditeState) {
	let Plans:Vec<Nudge> = Select(&ReadFrom(&State.PollWorker.Tasks))
		.into_iter()
		.filter_map(Render)
		.collect();

	Plans.into_iter().for_each(|Plan| PushNudge(State, &Plan.Text, Plan.TtlTurns));
}
