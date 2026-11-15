//! What the session does with an applied poll.
//!
//! A completion is stored and announced; a progress poll changes nothing outside
//! the task - the accumulator it grew is the whole effect - and an ignored poll
//! means the event named a task that was not running. The store and the marker are
//! `Interpret::Store`'s job, so this leaf is only the decision.

use crate::{
	PollWorker::{
		Apply::Applied::Struct::Applied,
		Completion::Build::Fn::Fn as Build,
		Transition::Enum::TaskTransition,
		Update::Interpret::Store::Fn::Fn as Store,
	},
	State::Struct::AphroditeState,
};

/// Store and announce a completed poll, or do nothing.
pub(crate) fn Fn(State:&mut AphroditeState, Applied:Applied) {
	let Index = Applied.Index;
	let Transition = Applied.Transition;
	let TaskID = Applied.TaskID.clone();

	let Completion = match Transition {
		TaskTransition::Completed { ExitCode } => Build(Applied, ExitCode),
		_ => return,
	};

	tracing::debug!(Task = %TaskID, Hash = %Completion.Hash, Size = Completion.Size, "poll worker: completion stored");

	Store(State, Index, Completion);
}
