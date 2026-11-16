//! The tracked record of one backgrounded tool call.
//!
//! This struct is the whole memory of the poll worker: identity, the poll
//! bookkeeping the nudge renderer reads, and the live-container progress surface.
//! It is not the output itself - completed output lives in the CCR store and only
//! its hash and preview are kept here, which is what keeps a long build from
//! pinning memory in the state.
//!
//! Two identities, deliberately distinct, and now two distinct types. `ID` is the
//! *command* identity: the CCR hash of the command line, stable across turns, and the
//! deduplication key, so two simultaneous runs of one command share it.
//! `ExecutionID` is the *execution* identity, learned from the first poll that names
//! the process; it is what lets a second poll of that process reach the same task.
//! Neither replaces the other, and neither can be stored in the other's slot.
//!
//! A finished task's stored output is not a pair of optional fields here: `BgStatus`
//! carries it, so "Done" without a hash is not a representable state.

use crate::{
	PollWorker::{Output::Struct::Accumulator, Task::Status::Enum::BgStatus},
	State::Identity::{Command::Struct::CommandID, Execution::Struct::ExecutionID},
};

/// A backgrounded tool call tracked by the poll worker.
#[derive(Debug, Clone)]
pub struct BgTask {
	/// Command identity: the CCR hash of the command string (stable across turns).
	pub ID:CommandID,
	/// Tool that produced the output: "terminal" or "process".
	pub Tool:String,
	/// The command being run (or tool + args summary for non-terminal).
	pub command:String,
	/// Turn on which the task was auto-backgrounded.
	pub StartedTurn:usize,
	/// Last turn the agent polled for this task.
	pub LastPollTurn:usize,
	/// Execution identity named by the first poll attributed to this task
	/// (`session_id` on the wire), once one has been seen.
	pub ExecutionID:Option<ExecutionID>,
	/// Current status, which owns the stored output's identity once the task has
	/// finished.
	pub Status:BgStatus,
	/// How many times the agent has polled.
	pub PollCount:usize,
	/// The uncompressed output content (accumulated from polls), bounded by the
	/// accumulator itself rather than by a length test at the call site.
	pub AccumulatedOutput:Accumulator,
	/// Live-container progress (pre-Rust `_hooks/live.py` container semantics):
	/// bytes of output arrived so far. Tracked on every task (cheap, real data)
	/// but only *surfaced* by the status renderer when `state.live_container` is
	/// on.
	pub LiveBytesArrived:usize,
	/// Estimated seconds until the task completes - while running, the
	/// elapsed-so-far heuristic (assume as long again as it has already taken; no
	/// output total is known for a backgrounded command); 0.0 once done/failed.
	pub LiveEtaS:f32,
	/// Unix-seconds timestamp of first arrival (task creation) - the poll-surface
	/// clock, shared with `crate::Prefetch::Clock::Fn::unix_now`.
	pub LiveArrivedAtUnix:u64,
}
