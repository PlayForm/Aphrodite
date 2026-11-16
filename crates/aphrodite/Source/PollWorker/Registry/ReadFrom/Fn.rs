//! Open the read-only view over a session's task deque.
//!
//! The render paths (`Status`, `Check`) hold `&AphroditeState` rather than a
//! registry, because their signatures are consumed outside this package. They
//! still read through the same view the registry hands out, so "how a selection
//! sees the tasks" has one definition.

use std::collections::VecDeque;

use crate::PollWorker::{Registry::Reader::Struct::Reader, Task::Struct::Struct as Record};

/// The read-only view of `Tasks`.
pub(crate) fn Fn(Tasks:&VecDeque<Record>) -> Reader<'_> { Reader { Tasks } }
