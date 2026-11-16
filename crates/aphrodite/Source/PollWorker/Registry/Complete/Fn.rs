//! Record the completion the session just stored.
//!
//! The hash, preview and size are the session's to compute (they need the store), so
//! this operation takes them - as one `StoredOutput`, not three loose arguments - and
//! owns everything else: the final byte count, the zeroed ETA and the status the exit
//! code implies.
//!
//! Recovery policy for the window `Registry::Poll` opens: between the poll that moved
//! the accumulator out and this call, the task is still `Running` with an empty
//! accumulator. Nothing can observe it - the two calls are one `UpdateFromPoll` on one
//! thread - but a panic between them would leave it there, and the answer is deliberate:
//! the task is written off by the expiry pass as stale, never as failed, and its output
//! (if the store had already taken it) stays retrievable under a hash nothing names. A
//! `None` here means the task record is gone; the caller says so on the log rather than
//! pretending the completion landed.

use crate::PollWorker::{
	Registry::Struct::Registry,
	Task::Status::{Exit::Fn::Fn as StatusOf, Stored::Struct::StoredOutput},
};

/// Record the stored output and the final status on the task at `Index`.
pub(crate) fn Fn(This:&mut Registry, Index:usize, Output:StoredOutput, ExitCode:i32) -> Option<()> {
	let Task = This.Tasks.get_mut(Index)?;

	// The full content has arrived: bytes-so-far is final and the ETA is zero.
	Task.LiveBytesArrived = Output.Size;
	Task.LiveEtaS = 0.0;
	Task.Status = StatusOf(ExitCode, Output);

	Some(())
}
