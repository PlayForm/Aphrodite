//! A poll that finished its task, with the output it moved out.

use crate::PollWorker::Apply::Applied::Subject::Struct::Subject;

/// A completion, before its output has been hashed and stored.
///
/// This type is the whole point of the `Outcome` split: it can only be built by the
/// poll that moved an accumulator out, so a function that prepares a completion takes
/// `Finished` and a progress result cannot reach it - that is a compile error, not a
/// runtime check. The output is OWNED here because it was moved out of the task's
/// accumulator; nothing copies a quarter megabyte to hand it on.
#[derive(Debug, Clone)]
pub(crate) struct Finished {
	/// The task that finished.
	pub Subject:Subject,
	/// The accumulator's buffer, moved out of the task.
	pub Output:String,
	/// The exit code the poll reported.
	pub ExitCode:i32,
}
