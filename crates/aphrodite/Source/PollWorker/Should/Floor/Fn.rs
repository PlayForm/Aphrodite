//! Is this output worth tracking?

use crate::PollWorker::Should::Floor::Const::MinimumTrackedBytes;

/// `true` when the output is large enough to be worth a poll worker.
pub(crate) fn Fn(Content:&str) -> bool { Content.len() >= MinimumTrackedBytes }
