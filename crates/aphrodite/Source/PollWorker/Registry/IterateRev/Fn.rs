//! Walk the tasks newest-first.
//!
//! Insertion order is the age order - the deque is appended to - so "newest first"
//! is a reverse walk, and the nudge pass depends on it: the task the agent most
//! recently backgrounded is the one most likely to still matter.

use crate::PollWorker::{Registry::Reader::Struct::Reader, Task::Struct::Struct as Record};

/// Every task, newest first.
pub(crate) fn Fn<'a>(This:&'a Reader<'_>) -> impl Iterator<Item = &'a Record> { This.Tasks.iter().rev() }
