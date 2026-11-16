//! One decision per task, equivalent to the two passes it replaces.
//!
//! The pass used to mark stale in place and then `retain` anything young enough, so
//! a task that became stale in this call could also be removed in this call if its
//! lifetime had run out. Combining the passes is only equivalent if the decision
//! sees the status the marking pass would have produced - which is what `Effective`
//! is - and the comparisons stay strict: stale means one turn *past* `Age`,
//! expired means one turn *past* `Lifetime`.
//!
//! `saturating_sub` is kept, so a turn counter that went backwards yields zero
//! rather than wrapping into an expiry.

use std::cmp::Ordering;

use crate::PollWorker::{
	Expire::{Const::Lifetime, Decision::Enum::Enum as Decision},
	Task::{Const::Age, Status::Enum::Enum as Status, Struct::Struct as Record},
};

/// The fate of `Task` at `Turn`.
pub(crate) fn Fn(Task:&Record, Turn:usize) -> Decision {
	// The status the marking pass would have given this task.
	let Effective = match &Task.Status {
		Status::Running => {
			match Turn.saturating_sub(Task.LastPollTurn).cmp(&Age) {
				Ordering::Greater => Status::Stale,
				Ordering::Equal | Ordering::Less => Status::Running,
			}
		},
		Other => Other.clone(),
	};

	match Effective {
		// A running task has no lifetime: it is kept however old it is, because the
		// agent may still poll it.
		Status::Running => Decision::Keep,
		_ => {
			match Turn.saturating_sub(Task.StartedTurn).cmp(&Lifetime) {
				Ordering::Greater => Decision::Remove,
				// Newly stale in this call, and young enough to keep the record.
				Ordering::Equal | Ordering::Less => {
					match Task.Status {
						Status::Running => Decision::MarkStale,
						_ => Decision::Keep,
					}
				},
			}
		},
	}
}
