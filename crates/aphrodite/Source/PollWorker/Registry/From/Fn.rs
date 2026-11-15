//! Open the mutable view over a session's task deque.

use std::collections::VecDeque;

use crate::PollWorker::{
	Registry::{Capacity::Struct::Capacity, Struct::Registry},
	Task::Struct::BgTask,
};

/// The registry over `Tasks`, at the shipped capacity.
pub(crate) fn Fn(Tasks:&mut VecDeque<BgTask>) -> Registry<'_> { Registry { Tasks, Bound:Capacity::Policy } }
