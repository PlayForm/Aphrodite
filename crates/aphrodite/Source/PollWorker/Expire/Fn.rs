//! The `PostLLMCall` expiry pass: write off tasks nobody is polling.
//!
//! Two separate clocks live here and they are not the same number. A running
//! task goes stale when the agent has not polled it for `StaleTurnAge`
//! turns; a finished task is dropped 16 turns after it started, so the agent
//! has a window to retrieve output the nudge already announced. Collapsing
//! the two into one threshold either kills a live build or forgets a
//! completion the agent was told about.

use crate::{
	PollWorker::Task::{Const::StaleTurnAge, Status::Enum::BgStatus},
	State::Struct::AphroditeState,
};

/// Expire background tasks that the agent has stopped polling.
/// Called from `PostLLMCall` after turn advancement.
pub fn ExpireStaleTasks(State:&mut AphroditeState) {
	let Turn = State.TurnCounter;
	for task in State.BgTasks.iter_mut() {
		if task.Status == BgStatus::Running && Turn.saturating_sub(task.LastPollTurn) > StaleTurnAge {
			task.Status = BgStatus::Stale;
		}
	}

	// Drop completed/failed/stale tasks older than 16 turns.
	State.BgTasks.retain(|task| {
		match task.Status {
			BgStatus::Running => true, // keep running tasks
			BgStatus::Done { .. } | BgStatus::Failed { .. } | BgStatus::Stale => {
				Turn.saturating_sub(task.StartedTurn) <= 16
			},
		}
	});
}
