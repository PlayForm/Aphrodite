//! Record a poll on one task: bookkeeping, output, and the transition.
//!
//! The task is mutated here and nowhere else for this step, so `LastPollTurn`,
//! `PollCount`, the accumulator and the live-container surface always move
//! together. On completion the accumulator is *moved* out into the returned
//! value - the old code cloned up to 256 KB per poll only to clear the original
//! on the next line (audit D7).
//!
//! The task's status is deliberately left alone: the completion needs a hash and
//! a preview the session computes, and `Registry::Complete` is the operation that
//! records them. Until it runs, the task is still running with its output moved
//! out, which is why `Complete` is always called in the same turn.

use crate::PollWorker::{
	Apply::Applied::Struct::Applied,
	Event::Struct::PollEvent,
	Output::Append::Fn::Fn as Append,
	Registry::Struct::Registry,
	Transition::{Decide::Fn::Fn as Decide, Enum::TaskTransition},
};

/// Record `Event` on the task at `Index`, reporting what it did.
pub(crate) fn Fn(This:&mut Registry, Index:usize, Event:&PollEvent, Turn:usize) -> Option<Applied> {
	let Task = This.Tasks.get_mut(Index)?;

	Task.LastPollTurn = Turn;
	Task.PollCount += 1;
	Task.ExecutionID = Event.ExecutionID.clone().or_else(|| Task.ExecutionID.clone());

	let Appended = Append(&mut Task.AccumulatedOutput, &Event.Output);

	// Live-container progress (pre-Rust `_hooks/live.py` container semantics):
	// bytes-so-far is the accumulated output; while still running, the ETA is the
	// elapsed-so-far heuristic - no output total is known for a backgrounded
	// command, so the estimate is that it takes as long again as it already has.
	Task.LiveBytesArrived = Task.AccumulatedOutput.Len();
	Task.LiveEtaS = Event.ObservedAtUnix.saturating_sub(Task.LiveArrivedAtUnix) as f32;

	tracing::trace!(Task = %Task.ID, Appended = ?Appended, "poll worker: output appended");

	let Transition = Decide(Task, Event);
	let Accumulated = match Transition {
		TaskTransition::Completed { .. } => Task.AccumulatedOutput.Take(),
		TaskTransition::Progressed | TaskTransition::Ignored => String::new(),
	};

	Some(Applied {
		Index,
		TaskID:Task.ID.clone(),
		Command:Task.command.clone(),
		Transition,
		Accumulated,
	})
}
