//! One `process(action='poll')` result, decoded.
//!
//! The wire has two shapes - the JSON envelope Hermes wraps a tool result in
//! and the raw text of one it did not - and both become this value before
//! anything is applied to a task. Decoding first is what lets the decision
//! (`Transition`) be a pure function of (task, event) instead of a function of
//! whichever string the hook happened to receive.
//!
//! `Command` and `ExecutionID` are two different identities and both are
//! optional: an envelope names the command it polled (`command`) and the
//! process it polled (`session_id`), a bare text result names neither. Neither
//! of them is the task identity - that is the command hash, which two runs of
//! one command share on purpose (see `Task::Struct`).

/// One poll result, decoded.
#[derive(Debug, Clone)]
pub(crate) struct PollEvent {
	/// The command line the poll result names, when the wire carries one.
	pub Command:Option<String>,
	/// The execution identity the poll result names (`session_id`), when the
	/// wire carries one. Recorded on the task it is first attributed to, so a
	/// later poll naming the same process reaches the same task.
	pub ExecutionID:Option<String>,
	/// Exit code, from the JSON envelope or from an `exit code: N` line.
	pub ExitCode:Option<i32>,
	/// The output text this poll contributes to the accumulator.
	pub Output:String,
	/// Unix seconds at which the poll was observed. Injected by the caller, so
	/// the elapsed-time surface is a function of its inputs.
	pub ObservedAtUnix:u64,
}
