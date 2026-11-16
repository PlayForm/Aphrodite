//! The read half of the registry.

use std::collections::VecDeque;

use crate::PollWorker::Task::Struct::Struct as Record;

/// A read-only view of the task set.
///
/// Selection and rendering need to see every task and change none of them. The
/// storage is visible so a selection function can walk it in place; there is no
/// mutable counterpart, which is what keeps "which task does this poll belong to"
/// and "which task makes room" from being able to mutate the set they inspect.
pub(crate) struct Reader<'a> {
	/// The storage, borrowed immutably.
	pub(crate) Tasks:&'a VecDeque<Record>,
}
