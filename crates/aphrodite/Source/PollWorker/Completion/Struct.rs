//! What a completed task's output is called and where it goes.
//!
//! The output is owned here, not borrowed: it was moved out of the task's
//! accumulator by the poll that completed it, so nothing copies a quarter megabyte
//! to hand it to the store. `Stored` is the same value the task's finished status
//! keeps, so the marker, the status and the nudge cannot disagree about the hash,
//! the preview or the size.

use crate::PollWorker::Task::Status::Stored::Struct::StoredOutput;

/// A completed poll worker's output, as the store and the marker need it.
#[derive(Debug, Clone)]
pub(crate) struct Completion {
	/// Where the output goes: hash, preview and byte size.
	pub Stored:StoredOutput,
	/// The stored output itself, moved out of the accumulator.
	pub Output:String,
	/// The command line the marker's centre names.
	pub Command:String,
	/// The exit code that finished the task.
	pub ExitCode:i32,
}
