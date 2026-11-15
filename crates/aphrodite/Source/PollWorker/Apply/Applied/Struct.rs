//! One poll's effect on one task, handed back to the session.

use crate::PollWorker::Transition::Enum::TaskTransition;

/// What applying a poll produced.
///
/// `Accumulated` is the task's buffer, moved out of it when the poll completed
/// the task - a progress poll leaves it empty and the task keeps accumulating.
#[derive(Debug, Clone)]
pub(crate) struct Applied {
	/// Index of the task the poll applied to.
	pub Index:usize,
	/// Command identity of that task: the CCR hash of its command line.
	pub TaskID:String,
	/// The command line, for the marker's centre label.
	pub Command:String,
	/// What the poll did to the task.
	pub Transition:TaskTransition,
	/// The accumulated output, moved out when the poll completed the task.
	pub Accumulated:String,
}
