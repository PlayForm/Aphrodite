//! Lifecycle state of one backgrounded task.
//!
//! `Running` is the only state in which the agent is expected to poll. A finished task
//! owns the identity of its stored output AND its outcome: `Done` carries no exit code
//! because success IS exit zero, and `Failed` carries a `NonZeroI32` because a failure
//! with code zero is not a thing. That is what makes "finished but no output recorded"
//! and "failed with exit 0" unrepresentable - the old shape kept a status beside two
//! `Option` fields and an `i32`, and the renderer had to invent a `?` for the case the
//! type allowed but nothing could produce.
//!
//! `Stale` is not a failure - it means the agent walked away, and the expiry pass
//! drops the task without ever claiming it failed.

use std::num::NonZeroI32;

use crate::PollWorker::Task::Status::Stored::Struct::StoredOutput;

/// Status of a backgrounded task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BgStatus {
	/// Task is still running - agent should keep polling.
	Running,
	/// Task completed successfully, with where its output was stored.
	Done {
		/// Where its output was stored.
		Output:StoredOutput,
	},
	/// Task failed, with the code that failed it and where its output was stored.
	Failed {
		/// The non-zero exit code that failed it.
		ExitCode:NonZeroI32,
		/// Where its output was stored.
		Output:StoredOutput,
	},
	/// Agent hasn't polled in `StaleTurnAge` turns - abandoned.
	Stale,
}
