//! The listener lifecycle: bind, watch, spawn, serve, drain.
//!
//! Every listener is bound before any of them serves, and that ordering is the
//! point: a port conflict used to surface only inside a spawned task, where it
//! logged an error and left that one listener silently dead while the other kept
//! answering - the process exited 0 and looked healthy.

mod Bind;
mod Drain;
mod Serve;
mod Shutdown;
mod Spawn;
mod Watcher;

pub use Bind::Bind;
pub use Drain::Drain;
pub use Serve::Serve;
pub use Shutdown::Signal;
pub use Spawn::Spawn;
pub use Watcher::watch;
