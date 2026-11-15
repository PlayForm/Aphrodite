//! Which task makes room when the registry is at capacity.
//!
//! `min_by_key` keeps the first of equal minima, which is what preserves
//! "oldest within a category": the rank says which category is expendable and
//! the index says which task in it went first. The victim is returned, not
//! logged - the caller decides whether an eviction is worth saying out loud.

use crate::PollWorker::{Registry::Reader::Struct::Reader, Select::Priority::Fn::Fn as Priority};

/// The task to evict, or `None` for an empty registry.
pub(crate) fn Fn(This:&Reader) -> Option<usize> {
	This.Tasks
		.iter()
		.enumerate()
		.min_by_key(|(Index, Task)| Priority(Task, *Index))
		.map(|(Index, _)| Index)
}
