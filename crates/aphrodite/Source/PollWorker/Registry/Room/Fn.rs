//! Decide what an admission finds.
//!
//! The duplicate test comes first and is exact: a task whose command identity
//! already exists *while running* is the same build reported twice, and the
//! existing task is already its record. Capacity is a three-way comparison rather
//! than a boolean, because "below the bound" and "at or over it" are the only two
//! states that matter and the ordering says which one it is.

use std::cmp::Ordering;

use crate::PollWorker::{
	Registry::{Capacity::Struct::Capacity, Reader::Struct::Reader, Room::Enum::Room},
	Select::Eviction::Fn::Fn as Eviction,
	Task::Status::Enum::BgStatus,
};

/// What admitting a task with `Identity` finds.
pub(crate) fn Fn(This:&Reader, Identity:&str, Bound:Capacity) -> Room {
	match This
		.Tasks
		.iter()
		.position(|Task| Task.ID == Identity && Task.Status == BgStatus::Running)
	{
		Some(_) => Room::Duplicate,
		None => {
			match This.Tasks.len().cmp(&Bound.Get()) {
				Ordering::Less => Room::Free,
				Ordering::Equal | Ordering::Greater => Room::Evict(Eviction(This)),
			}
		},
	}
}
