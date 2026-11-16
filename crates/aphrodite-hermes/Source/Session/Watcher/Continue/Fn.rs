//! Whether one apply pass is a reason to keep watching.

use crate::Session::Watcher::Outcome;

/// The "keep watching?" decision, in one place and as a function of the pass.
///
/// The sweep reports counts; this is the only leaf that turns them into a
/// verdict, which is what keeps the decision reviewable and lets the sweep stay
/// a report. Every branch here is a case the old `bool` could not express:
///
///   - a parse failure keeps watching, because the next successful write is
///     what clears it (the previous values are still in force);
///   - a pass that reached at least one session keeps watching while any of
///     them still wants a reload;
///   - a pass that reached NOBODY keeps watching. `Applied == 0` means either
///     the registry was empty (a sweep that raced the first session) or every
///     session's lock was wedged (the sweep could not look). Neither is the file
///     saying "stop", and the old code stopped on both.
pub(crate) fn Fn(Pass:&Outcome::Enum::Enum) -> bool {
	match Pass {
		Outcome::Enum::Enum::Failed(_) => true,
		Outcome::Enum::Enum::Applied(Report) => Report.WantsReload > 0 || Report.Applied == 0,
	}
}
