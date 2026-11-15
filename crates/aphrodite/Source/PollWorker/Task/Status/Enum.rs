//! Lifecycle state of one backgrounded task.
//!
//! `Running` is the only state in which the agent is expected to poll;
//! `Done` and `Failed` carry the real exit code so the nudge can tell the
//! agent whether to retrieve output or investigate a failure. `Stale` is
//! not a failure - it means the agent walked away, and the expiry pass drops
//! the task without ever claiming it failed.

/// Status of a backgrounded task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BgStatus {
	/// Task is still running - agent should keep polling.
	Running,
	/// Task completed successfully.
	Done { ExitCode:i32 },
	/// Task failed.
	Failed { ExitCode:i32 },
	/// Agent hasn't polled in `StaleTurnAge` turns - abandoned.
	Stale,
}
