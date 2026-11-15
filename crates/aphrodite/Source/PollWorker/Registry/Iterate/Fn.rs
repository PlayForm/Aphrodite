//! Walk the tasks oldest-first.

use crate::PollWorker::{Registry::Reader::Struct::Reader, Task::Struct::BgTask};

/// Every task, in insertion order.
pub(crate) fn Fn<'a>(This:&'a Reader<'_>) -> impl Iterator<Item = &'a BgTask> { This.Tasks.iter() }
