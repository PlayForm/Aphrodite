//! The listener lifecycle: bind, watch, spawn, serve, drain.
//!
//! Every listener is bound before any of them serves, and that ordering is the
//! point: a port conflict used to surface only inside a spawned task, where it
//! logged an error and left that one listener silently dead while the other kept
//! answering - the process exited 0 and looked healthy.

pub mod Bind;
pub mod Drain;
pub mod Serve;
pub mod Shutdown;
pub mod Spawn;
pub mod Watcher;

