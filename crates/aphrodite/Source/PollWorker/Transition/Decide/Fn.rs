//! Decide what a poll did, without touching anything.
//!
//! The inputs are the task and the event, both already decoded: the observed
//! time and the turn were read once by the caller and carried in the event, so
//! this function is reproducible and a test can pin the elapsed-time surface.

use crate::PollWorker::{
	Event::Struct::PollEvent,
	Task::{Status::Enum::BgStatus, Struct::BgTask},
	Transition::Enum::TaskTransition,
};

/// What this poll did to this task.
pub(crate) fn Fn(Task:&BgTask, Event:&PollEvent) -> TaskTransition {
	match Task.Status {
		BgStatus::Running => {
			match Event.ExitCode {
				Some(ExitCode) => TaskTransition::Completed { ExitCode },
				None => TaskTransition::Progressed,
			}
		},
		// A task that is not running has nothing left to progress or complete.
		BgStatus::Done { .. } | BgStatus::Failed { .. } | BgStatus::Stale => TaskTransition::Ignored,
	}
}
