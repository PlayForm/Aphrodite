//! Poll-result ingestion: decode the wire, apply it to one task, interpret the
//! outcome.
//!
//! The observed time is read once, here, and injected into the event - nothing
//! below this line reads the clock, so the elapsed-time surface the live container
//! renders is a function of its inputs.
//!
//! Returns `true` when the poll was consumed (it applied to a tracked task), so the
//! caller skips normal compression; `false` lets the result pass through as
//! ordinary output.
//!
//! # The one chosen recovery policy
//!
//! Moving the accumulator out and recording the completion are two phases because the
//! second needs `&mut AphroditeState` and the first holds the registry's borrow. They
//! are kept inside THIS function - one synchronous call, one thread, no lock released
//! between them - which is the policy the two-phase split requires: a `VecDeque` index
//! is never carried across an interval in which the set could change, so the index the
//! completion is recorded against still names the task the poll applied to. The panic
//! case (a task left running with its accumulator handed off) is written off by the
//! expiry pass as stale, never as failed - see `Registry::Complete`.

use crate::{
	PollWorker::{
		Apply::{Applied::Enum::Applied, Fn::Fn as Apply},
		Decode::Fn::Fn as Decode,
		Registry::From::Fn::Fn as Registry,
		Update::Interpret::Fn::Fn as Interpret,
	},
	State::Struct::AphroditeState,
};

/// Update a background task from a `process(action='poll')` result.
///
/// `true` when the poll was consumed - it applied to a tracked running task, whether it
/// progressed or completed it. `false` means the result names no task of ours and the
/// caller takes the normal compression path.
pub fn UpdateFromPoll(State:&mut AphroditeState, ToolName:&str, Content:&str) -> bool {
	let ObservedAtUnix = crate::Prefetch::Clock::Fn::UnixNow();
	let Turn = State.TurnCounter;

	let Applied:Option<Applied> = Decode(ToolName, Content, ObservedAtUnix).and_then(|Event| {
		let mut Tasks = Registry(&mut State.BgTasks);

		Apply(&mut Tasks, &Event, Turn)
	});

	match Applied {
		None => false,
		Some(Applied) => {
			Interpret(State, Applied);

			true
		},
	}
}
