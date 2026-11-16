//! Reading a status's stored output, when it has one.

use crate::PollWorker::Task::Status::{Enum::Enum as Status, Stored::Struct::Struct as Payload};

/// The stored output of a finished task, or `None` while it is running or abandoned.
///
/// The status is the only owner, so this is the one way to reach the hash - a caller
/// that wants to name output has to have proved the task finished first. A `?` on this
/// answer is the honest form of "no output exists yet"; an empty-string or `None` field
/// on the task was not.
pub(crate) fn Fn(Status:&Status) -> Option<&Payload> {
	match Status {
		Status::Done { Output, .. } | Status::Failed { Output, .. } => Some(Output),
		Status::Running | Status::Stale => None,
	}
}
