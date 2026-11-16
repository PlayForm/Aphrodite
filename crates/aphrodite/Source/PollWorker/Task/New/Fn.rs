//! The constructor for a tracked task.
//!
//! One place that knows what a fresh task looks like, so the live-container clock
//! and the accumulator bound are set once instead of being restated by every
//! caller that admits a task.

use crate::{
	PollWorker::{
		Output::Struct::Accumulator,
		Task::{Status::Enum::BgStatus, Struct::BgTask},
	},
	State::Identity::Command::Struct::CommandID,
};

/// A running task, started at `Turn`, first observed at `ObservedAtUnix`.
pub(crate) fn Fn(TaskID:CommandID, Tool:String, command:String, Turn:usize, ObservedAtUnix:u64) -> BgTask {
	BgTask {
		ID:TaskID,
		Tool,
		command,
		StartedTurn:Turn,
		LastPollTurn:Turn,
		ExecutionID:None,
		Status:BgStatus::Running,
		PollCount:0,
		AccumulatedOutput:Accumulator::New(),
		LiveBytesArrived:0,
		LiveEtaS:0.0,
		LiveArrivedAtUnix:ObservedAtUnix,
	}
}
