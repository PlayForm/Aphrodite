//! The three answers a poll can give about a task.

/// What one decoded poll did to one task.
///
/// The session decides what to do with each answer: `Completed` is stored and
/// announced, `Progressed` changes nothing outside the task, `Ignored` means the
/// poll named a task that was not running. Keeping the answer as a value is what
/// lets the storing and the announcing live in the session layer while the
/// decision stays a pure function of (task, event).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskTransition {
	/// The poll carried output but no exit code: the task is still running.
	Progressed,
	/// The poll carried the exit code: the task is finished with it.
	Completed { ExitCode:i32 },
	/// The poll does not apply to this task at all.
	Ignored,
}
