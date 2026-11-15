//! Admitting a new task into the tracked set, under a hard capacity bound.
//!
//! Two rules, both about not losing the wrong thing: a task whose command is
//! already running is not tracked twice (the hash is the identity, so a
//! duplicate would be the same build reported twice), and at capacity the
//! set evicts in priority order - stale first, then finished, and only then
//! the oldest still-running task. Evicting a running task is the last resort
//! because it silently orphans work the agent still expects to poll.

use crate::{
	PollWorker::Task::{Const::MaximumBgTasks, Status::Enum::BgStatus, Struct::BgTask},
	State::Struct::AphroditeState,
};

/// Insert a new background task into the state, evicting the oldest
/// completed/stale task if at capacity.
pub fn InsertBgTask(State:&mut AphroditeState, TaskID:String, Tool:String, command:String, Turn:usize) {
	// Deduplicate: don't track the same command twice while it's running.
	match State.BgTasks.iter().any(|t| t.ID == TaskID && t.Status == BgStatus::Running) {
		true => {
			return;
		},
		false => {},
	}

	// Evict oldest non-running task if at capacity.
	loop {
		match State.BgTasks.len() >= MaximumBgTasks {
			true => {
				// Prefer evicting stale, then done/failed.
				let StalePos = State.BgTasks.iter().position(|t| t.Status == BgStatus::Stale);
				let DonePos = State
					.BgTasks
					.iter()
					.position(|t| matches!(t.Status, BgStatus::Done { .. } | BgStatus::Failed { .. }));
				let Pos = StalePos.or(DonePos);
				match Pos {
					Some(p) => {
						State.BgTasks.remove(p);
					},
					None => {
						// All running - drop the oldest running task.
						State.BgTasks.remove(0);
					},
				}
			},
			false => break,
		}
	}

	State.BgTasks.push_back(BgTask {
		ID:TaskID,
		Tool,
		command,
		StartedTurn:Turn,
		LastPollTurn:Turn,
		Status:BgStatus::Running,
		OutputHash:None,
		OutputPreview:None,
		PollCount:0,
		AccumulatedOutput:String::new(),
		LiveBytesArrived:0,
		LiveEtaS:0.0,
		LiveArrivedAtUnix:crate::Prefetch::Clock::Fn::UnixNow(),
	});
}
