//! The poll-worker auto-backgrounding domain's own fields.
//!
//! Split out of `AphroditeState` so the poll-worker registry, insert, expire and
//! status paths take `&mut PollWorker::Struct` or `&PollWorker::Struct`: the
//! background-task deque and the master switch are one domain, and the expiry
//! sweep needs nothing else the session carries.

use std::collections::VecDeque;

use crate::PollWorker::Task::Struct::Struct as Task;

/// The background-task deque and the master on/off switch that fills it.
#[derive(Clone)]
pub struct Struct {
	/// Background tasks created by the poll-worker auto-backgrounding
	/// heuristic (cap 4, evict oldest completed/stale on overflow).
	pub BgTasks:VecDeque<Task>,
	/// Master on/off for poll-worker auto-backgrounding. When false,
	/// no tool output is auto-backgrounded (existing bg_tasks still
	/// receive lifecycle nudges and expiry). Default true. Env:
	/// `APHRODITE_POLL_WORKER`, TOML: `[compression] poll_worker`.
	pub PollWorkerEnabled:bool,
}
