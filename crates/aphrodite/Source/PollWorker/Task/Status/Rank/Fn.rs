//! How expendable a task in a given status is.
//!
//! The order is policy, not incidental: a stale task is one the agent already
//! walked away from, a finished one has been announced, and a running one is work
//! the agent still expects to poll - so evicting a running task is the last
//! resort it always was.

use crate::PollWorker::Task::Status::Enum::BgStatus;

/// Eviction rank: lower is evicted sooner.
pub(crate) fn Fn(Status:&BgStatus) -> u8 {
	match Status {
		BgStatus::Stale => 0,
		BgStatus::Done { .. } | BgStatus::Failed { .. } => 1,
		BgStatus::Running => 2,
	}
}
