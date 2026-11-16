//! `Prepare`: a command with its external effects performed.

use super::super::{Command::Enum::Enum as Command, Prepared::Enum::Enum as Prepared};

/// Perform whatever a command must do before the session lock is taken.
///
/// `prefetch` reads up to ten files of up to 10 MB each, so it is the one
/// command whose work must not happen under the lock: it ran under the global
/// handle-map mutex once, which serialised every other session's every call
/// behind one slow or cold mount. Every other command needs nothing done to it
/// and passes through as `Ready`.
pub(crate) fn Fn(Command:Command<'_>) -> Prepared<'_> {
	match Command {
		Command::Prefetch { Paths } => Prepared::Prefetch { Outcomes:crate::Prefetch::Read::Entry::Fn::Fn(&Paths) },
		Other => Prepared::Ready(Other),
	}
}
