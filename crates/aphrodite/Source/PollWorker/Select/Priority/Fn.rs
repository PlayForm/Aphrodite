//! The priority key of one candidate.

use crate::PollWorker::{
	Select::Priority::Struct::Priority,
	Task::{Status::Rank::Fn::Fn as Rank, Struct::Struct as Record},
};

/// The eviction key for a task at a given position.
pub(crate) fn Fn(Task:&Record, Index:usize) -> Priority { Priority { Rank:Rank(&Task.Status), Index } }
