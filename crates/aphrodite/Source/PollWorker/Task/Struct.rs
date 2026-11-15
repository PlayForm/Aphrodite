//! The tracked record of one backgrounded tool call.
//!
//! This struct is the whole memory of the poll worker: identity (a CCR hash
//! of the command, so the same command across turns resolves to the same
//! task), the poll bookkeeping the nudge renderer reads, and the
//! live-container progress surface. It is not the output itself - completed
//! output lives in the CCR store and only its hash and preview are kept
//! here, which is what keeps a long build from pinning memory in the state.

use crate::PollWorker::Task::Status::Enum::BgStatus;

/// A backgrounded tool call tracked by the poll worker.
#[derive(Debug, Clone)]
pub struct BgTask {
	/// Unique ID: CCR hash of the command string (stable across turns).
	pub ID:String,
	/// Tool that produced the output: "terminal" or "process".
	pub Tool:String,
	/// The command being run (or tool + args summary for non-terminal).
	pub command:String,
	/// Turn on which the task was auto-backgrounded.
	pub StartedTurn:usize,
	/// Last turn the agent polled for this task.
	pub LastPollTurn:usize,
	/// Current status.
	pub Status:BgStatus,
	/// CCR hash of the compressed output (set when Done/Failed).
	pub OutputHash:Option<String>,
	/// CCR preview of the compressed output (set when Done/Failed).
	pub OutputPreview:Option<String>,
	/// How many times the agent has polled.
	pub PollCount:usize,
	/// The uncompressed output content (accumulated from polls).
	/// Capped to avoid memory blow-up from long builds.
	pub AccumulatedOutput:String,
	/// Live-container progress (pre-Rust `_hooks/live.py` container
	/// semantics): bytes of output arrived so far. Tracked on every task
	/// (cheap, real data) but only *surfaced* by the status renderer when
	/// `state.live_container` is on.
	pub LiveBytesArrived:usize,
	/// Estimated seconds until the task completes - while running, the
	/// elapsed-so-far heuristic (assume as long again as it has already
	/// taken; no output total is known for a backgrounded command); 0.0
	/// once done/failed.
	pub LiveEtaS:f32,
	/// Unix-seconds timestamp of first arrival (task creation) - the
	/// poll-surface clock, shared with `crate::Prefetch::Clock::Fn::unix_now`.
	pub LiveArrivedAtUnix:u64,
}
