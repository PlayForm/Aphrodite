//! The priority key of one candidate.

use crate::PollWorker::{
	Select::Priority::Struct::Priority,
	Task::{Status::Rank::Fn::Fn as Rank, Struct::BgTask},
};

/// The eviction key for a task at a given position.
pub(crate) fn Fn(Task:&BgTask, Index:usize) -> Priority { Priority { Rank:Rank(&Task.Status), Index } }
