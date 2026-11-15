//! Turn an applied completion into a storeable value.
//!
//! The output is moved, never cloned: it is the accumulator's buffer, up to
//! 256 KB, and the old code copied it once per completion only to clear the
//! original on the next line.

use crate::PollWorker::{Apply::Applied::Struct::Applied, Completion::Struct::Completion};

/// Hash, preview and size of a completed task's output.
pub(crate) fn Fn(Applied:Applied, ExitCode:i32) -> Completion {
	let Size = Applied.Accumulated.len();
	let Hash = headroom_core::ccr::compute_key(Applied.Accumulated.as_bytes());
	let Preview = crate::Preview::Builder::Entry::Fn("terminal", &Applied.Accumulated);

	Completion {
		Hash,
		Output:Applied.Accumulated,
		Preview,
		Size,
		Command:Applied.Command,
		ExitCode,
	}
}
