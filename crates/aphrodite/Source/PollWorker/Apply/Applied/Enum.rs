//! The two outcomes a poll can have on a tracked task.

use crate::PollWorker::Apply::Applied::{Finished::Struct::Finished, Subject::Struct::Subject};

/// What applying a poll produced.
///
/// Two variants, not three: `Progressed` carries only the task it moved, and
/// `Completed` carries the moved-out output and the exit code that finished it. The
/// exit code is not an `Option` here and the output is not an empty string on the
/// progress path - each variant holds exactly what its case has.
#[derive(Debug, Clone)]
pub(crate) enum Applied {
	/// The poll carried output but no exit code: the task is still running, and its
	/// accumulator grew by whatever fitted.
	Progressed(Subject),
	/// The poll carried the exit code: the task is finished with it, and the
	/// accumulator's buffer has been moved out into this value.
	Completed(Finished),
}
