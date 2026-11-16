//! Record a poll on one task: bookkeeping, output, and the outcome.
//!
//! The task is mutated here and nowhere else for this step, so `LastPollTurn`,
//! `PollCount`, the accumulator and the live-container surface always move
//! together. On completion the accumulator is *moved* out into the returned value -
//! the old code cloned up to 256 KB per poll only to clear the original on the next
//! line (audit D7).
//!
//! The task's status is deliberately left alone: the completion needs a hash and a
//! preview the session computes, and `Registry::Complete` is the operation that
//! records them. Until it runs, the task is still running with its output moved out,
//! which is why `Complete` is always called in the same turn - and why the recovery
//! policy for that window is stated in `Registry::Complete`.
//!
//! The exit code is the whole decision. A running task that reports one has
//! finished; one that reports none has only progressed. There is no third outcome to
//! report here, because a poll that names no running task of ours never reaches this
//! function - `Select::Target` is the only selector and it filters on `Running`, and
//! `Apply::Fn` answers `None` when it finds nothing.

use crate::PollWorker::{
	Apply::Applied::{Enum::Enum as Outcome, Finished::Struct::Finished, Subject::Struct::Subject},
	Event::Struct::Struct as Wire,
	Output::Append::Fn::Fn as Append,
	Registry::Struct::Registry,
};

/// Record `Event` on the task at `Index`, reporting what it did.
///
/// The caller must have selected a RUNNING task: this function cannot check it
/// without re-reading a status the selection already filtered, and a status check here
/// would be a second attribution rule.
pub(crate) fn Fn(This:&mut Registry, Index:usize, Event:&Wire<'_>, Turn:usize) -> Option<Outcome> {
	let Task = This.Tasks.get_mut(Index)?;

	Task.LastPollTurn = Turn;
	Task.PollCount += 1;
	Task.Execution = Event.Execution.clone().or_else(|| Task.Execution.clone());

	let Appended = Append(&mut Task.AccumulatedOutput, Event.Output.as_ref());

	// Live-container progress (pre-Rust `_hooks/live.py` container semantics):
	// bytes-so-far is the accumulated output; while still running, the ETA is the
	// elapsed-so-far heuristic - no output total is known for a backgrounded
	// command, so the estimate is that it takes as long again as it already has.
	Task.LiveBytesArrived = Task.AccumulatedOutput.Len();
	Task.LiveEtaS = Event.ObservedAtUnix.saturating_sub(Task.LiveArrivedAtUnix) as f32;

	tracing::trace!(Task = %Task.ID, Appended = ?Appended, "poll worker: output appended");

	let Subject = Subject { Index, ID:Task.ID.clone(), Command:Task.command.clone() };

	Some(match Event.ExitCode {
		Some(ExitCode) => Outcome::Completed(Finished { Subject, Output:Task.AccumulatedOutput.Take(), ExitCode }),
		None => Outcome::Progressed(Subject),
	})
}
