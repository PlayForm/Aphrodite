//! The status an exit code implies.
//!
//! Zero is success and everything else is failure - the same split the completion
//! path made by hand, named once so the marker, the status and the nudge cannot
//! disagree about what "exit 0" means.

use crate::PollWorker::Task::Status::Enum::BgStatus;

/// `Done` for a zero exit code, `Failed` otherwise.
pub(crate) fn Fn(ExitCode:i32) -> BgStatus {
	match ExitCode {
		0 => BgStatus::Done { ExitCode:0 },
		Code => BgStatus::Failed { ExitCode:Code },
	}
}
