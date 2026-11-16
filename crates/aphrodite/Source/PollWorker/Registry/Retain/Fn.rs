//! Apply one retention decision per task, reporting what each one did.
//!
//! The decision is a parameter, not a policy of this function: the expiry pass owns
//! the thresholds, the registry owns the mutation and the bookkeeping. A task kept
//! after being marked stale keeps its new status, and a removed task is named in
//! the report before it goes.

use crate::PollWorker::{
	Expire::{Decision::Enum::Enum as Decision, Report::Struct::Struct as Expiry},
	Registry::Struct::Registry,
	Task::{Status::Enum::Enum as Status, Struct::Struct as Record},
};

/// Walk the set once, applying `Decide` to each task.
pub(crate) fn Fn(This:&mut Registry, mut Decide:impl FnMut(&mut Record) -> Decision) -> Expiry {
	let mut Report = Expiry::default();

	This.Tasks.retain_mut(|Task| {
		match Decide(Task) {
			Decision::Keep => true,
			Decision::MarkStale => {
				Report.MarkedStale.push(Task.ID.clone());
				Task.Status = Status::Stale;
				true
			},
			Decision::Remove => {
				Report.Removed.push(Task.ID.clone());
				false
			},
		}
	});

	Report
}
