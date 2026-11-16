//! The `PostLLMCall` expiry entry point.
//!
//! Called from `Hook::LLM::Post` after the turn counter advances, so the age
//! comparisons see the turn the agent is about to be told about. The report is
//! available to the caller that wants it; this entry point does not consume it,
//! because nothing on the turn path acts on an expiry beyond the removal itself.

use crate::{
	PollWorker::{Expire::Apply::Fn::Fn as Apply, Registry::From::Fn::Fn as Registry},
	State::Struct::AphroditeState,
};

/// Expire background tasks that the agent has stopped polling.
pub fn Fn(State:&mut AphroditeState) {
	let Turn = State.TurnCounter;
	let mut Tasks = Registry(&mut State.BgTasks);

	let _Report = Apply(&mut Tasks, Turn);
}
