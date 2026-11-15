//! The constructor for a tracked task.
//!
//! One place that knows what a fresh task looks like, so the live-container clock
//! and the accumulator bound are set once instead of being restated by every
//! caller that admits a task.

use crate::PollWorker::{
	Output::Struct::Accumulator,
	Task::{Status::Enum::BgStatus, Struct::BgTask},
};

/// A running task, started at `Turn`, first observed at `ObservedAtUnix`.
pub(crate) fn Fn(TaskID:String, Tool:String, command:String, Turn:usize, ObservedAtUnix:u64) -> BgTask {
	BgTask {
		ID:TaskID,
		Tool,
		command,
		StartedTurn:Turn,
		LastPollTurn:Turn,
		ExecutionID:None,
		Status:BgStatus::Running,
		OutputHash:None,
		OutputPreview:None,
		PollCount:0,
		AccumulatedOutput:Accumulator::New(),
		LiveBytesArrived:0,
		LiveEtaS:0.0,
		LiveArrivedAtUnix:ObservedAtUnix,
	}
}
