//! The mutable view over the session's task deque.

use std::collections::VecDeque;

use crate::PollWorker::{Registry::Capacity::Struct::Capacity, Task::Struct::BgTask};

/// The task set, with its capacity.
///
/// Every mutation goes through an operation in this module's directory
/// (`Admit`, `Poll`, `Complete`, `Retain`), so "at most `Bound` tasks", "never
/// two running tasks with one identity" and "the victim is the oldest expendable
/// task" are enforced in one place instead of being re-derived by each caller.
pub(crate) struct Registry<'a> {
	/// The storage. Visible inside `Registry` only: outside, the operations are
	/// the surface.
	pub(super) Tasks:&'a mut VecDeque<BgTask>,
	/// The bound this registry keeps, non-zero by construction.
	pub(super) Bound:Capacity,
}
