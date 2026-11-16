//! The constructor for a tracked task.
//!
//! One place that knows what a fresh task looks like, so the live-container clock
//! and the accumulator bound are set once instead of being restated by every
//! caller that admits a task.

use crate::{
	PollWorker::{
		Output::Struct::Accumulator,
		Task::{Status::Enum::Enum as Status, Struct::Struct as Record},
	},
	State::Identity::Command::Struct::Struct as Command,
};

/// A running task, started at `Turn`, first observed at `ObservedAtUnix`.
pub(crate) fn Fn(TaskID:Command, Tool:String, command:String, Turn:usize, ObservedAtUnix:u64) -> Record {
	Record {
		ID:TaskID,
		Tool,
		command,
		StartedTurn:Turn,
		LastPollTurn:Turn,
		Execution:None,
		Status:Status::Running,
		PollCount:0,
		AccumulatedOutput:Accumulator::New(),
		LiveBytesArrived:0,
		LiveEtaS:0.0,
		LiveArrivedAtUnix:ObservedAtUnix,
	}
}
