//! Turn an applied completion into a storeable value.
//!
//! The argument is `Finished`, not `Applied`: a progress result cannot reach this
//! function at all, which is the compile-time half of "a poll that did not finish
//! cannot prepare a completion". The output is moved, never cloned - it is the
//! accumulator's buffer, up to 256 KB, and the old code copied it once per completion
//! only to clear the original on the next line.

use crate::{
	PollWorker::{
		Apply::Applied::Finished::Struct::Finished,
		Completion::Struct::Completion,
		Task::Status::Stored::Struct::StoredOutput,
	},
	State::Identity::Content::Struct::ContentHash,
};

/// Hash, preview and size of a completed task's output, plus where it goes.
pub(crate) fn Fn(Finished:Finished) -> Completion {
	let Size = Finished.Output.len();
	let Hash = ContentHash::from(headroom_core::ccr::compute_key(Finished.Output.as_bytes()));
	let Preview = crate::Preview::Builder::Entry::Fn("terminal", &Finished.Output);

	Completion {
		Stored:StoredOutput { Hash, Preview, Size },
		Output:Finished.Output,
		Command:Finished.Subject.Command,
		ExitCode:Finished.ExitCode,
	}
}
