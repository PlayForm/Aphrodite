//! The task set, and the operations that are allowed to change it.
//!
//! The registry is a borrowing view over the session's `PollWorker::Tasks` deque: the
//! field's type is fixed outside this package, so the registry owns the *rules*
//! rather than the storage. Capacity, deduplication, eviction order and the
//! poll/complete/expire transitions all live here; nothing outside pushes,
//! removes or retains directly, because those are the decisions that used to be
//! spread across four modules.
//!
//! `Reader` is the read half: a selection function can look at every task and
//! change none of them.

pub mod Admit;
pub mod Capacity;
pub mod Complete;
pub mod From;
pub mod Iterate;
pub mod IterateRev;
pub mod Poll;
pub mod Read;
pub mod ReadFrom;
pub mod Reader;
pub mod Retain;
pub mod Room;
pub mod Struct;
