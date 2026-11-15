//! The poll-worker line.

use crate::State::Struct::AphroditeState;

/// How many poll workers are still running, when the feature is on and any are.
///
/// The line names the verb the agent should use (`process(action='poll')`)
/// because a running task is useless information without it: the model would
/// otherwise wait for a completion notification that never arrives.
pub(crate) fn Fn(State:&AphroditeState) -> Option<String> {
	let Active = State
		.BgTasks
		.iter()
		.filter(|Task| Task.Status == crate::PollWorker::Task::Status::Enum::BgStatus::Running)
		.count();

	(State.PollWorkerEnabled && Active > 0)
		.then(|| format!("{Active} active poll worker(s). Use process(action='poll')."))
}
