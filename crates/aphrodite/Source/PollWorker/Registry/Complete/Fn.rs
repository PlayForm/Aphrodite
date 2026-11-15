//! Record the completion the session just stored.
//!
//! The hash and preview are the session's to compute (they need the store), so
//! this operation takes them and owns everything else: the output reference, the
//! final byte count, the zeroed ETA and the status the exit code implies.

use crate::PollWorker::{Registry::Struct::Registry, Task::Status::Exit::Fn::Fn as StatusOf};

/// Record the stored output and the final status on the task at `Index`.
pub(crate) fn Fn(This:&mut Registry, Index:usize, Hash:&str, Preview:&str, Size:usize, ExitCode:i32) -> Option<()> {
	let Task = This.Tasks.get_mut(Index)?;

	Task.OutputHash = Some(Hash.to_string());
	Task.OutputPreview = Some(Preview.to_string());
	// The full content has arrived: bytes-so-far is final and the ETA is zero.
	Task.LiveBytesArrived = Size;
	Task.LiveEtaS = 0.0;
	Task.Status = StatusOf(ExitCode);

	Some(())
}
