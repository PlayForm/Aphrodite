//! The status an exit code implies.

use std::num::NonZeroI32;

use crate::PollWorker::Task::Status::{Enum::BgStatus, Stored::Struct::StoredOutput};

/// `Done` for exit zero, `Failed` for any other code - carrying where the output went.
///
/// The split is the type's, not a convention: `NonZeroI32::new` answering `None` IS
/// "success", so `Failed` cannot hold a zero code and `Done` has no code to hold. One
/// place names it, so the marker, the status and the nudge cannot disagree about what
/// "exit 0" means.
pub(crate) fn Fn(ExitCode:i32, Output:StoredOutput) -> BgStatus {
	match NonZeroI32::new(ExitCode) {
		None => BgStatus::Done { Output },
		Some(ExitCode) => BgStatus::Failed { ExitCode, Output },
	}
}
