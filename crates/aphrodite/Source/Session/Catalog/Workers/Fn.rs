//! The poll-worker line.

use std::collections::VecDeque;

use crate::PollWorker::Task::{Status::Enum::Enum as Status, Struct::Struct as Record};

/// How many poll workers are still running, when the feature is on and any are.
///
/// Takes the flag and the set it reads rather than the session: this section reports
/// on the poll worker and cannot reach anything else.
///
/// The line names the verb the agent should use (`process(action='poll')`)
/// because a running task is useless information without it: the model would
/// otherwise wait for a completion notification that never arrives.
pub(crate) fn Fn(Enabled:bool, Tasks:&VecDeque<Record>) -> Option<String> {
	let Active = Tasks.iter().filter(|Task| Task.Status == Status::Running).count();

	(Enabled && Active > 0).then(|| format!("{Active} active poll worker(s). Use process(action='poll')."))
}
