//! Walk the tasks oldest-first.

use crate::PollWorker::{Registry::Reader::Struct::Reader, Task::Struct::Struct as Record};

/// Every task, in insertion order.
pub(crate) fn Fn<'a>(This:&'a Reader<'_>) -> impl Iterator<Item = &'a Record> { This.Tasks.iter() }
