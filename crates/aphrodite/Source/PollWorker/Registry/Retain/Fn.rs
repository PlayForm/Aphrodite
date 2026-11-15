//! Apply one retention decision per task, reporting what each one did.
//!
//! The decision is a parameter, not a policy of this function: the expiry pass owns
//! the thresholds, the registry owns the mutation and the bookkeeping. A task kept
//! after being marked stale keeps its new status, and a removed task is named in
//! the report before it goes.

use crate::PollWorker::{
	Expire::{Decision::Enum::RetentionDecision, Report::Struct::ExpirationReport},
	Registry::Struct::Registry,
	Task::{Status::Enum::BgStatus, Struct::BgTask},
};

/// Walk the set once, applying `Decide` to each task.
pub(crate) fn Fn(This:&mut Registry, mut Decide:impl FnMut(&mut BgTask) -> RetentionDecision) -> ExpirationReport {
	let mut Report = ExpirationReport::default();

	This.Tasks.retain_mut(|Task| {
		match Decide(Task) {
			RetentionDecision::Keep => true,
			RetentionDecision::MarkStale => {
				Report.MarkedStale.push(Task.ID.clone());
				Task.Status = BgStatus::Stale;
				true
			},
			RetentionDecision::Remove => {
				Report.Removed.push(Task.ID.clone());
				false
			},
		}
	});

	Report
}
