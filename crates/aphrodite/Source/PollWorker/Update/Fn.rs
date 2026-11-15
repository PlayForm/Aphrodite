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

use crate::{
	PollWorker::{
		Apply::{Applied::Struct::Applied, Fn::Fn as Apply},
		Decode::Fn::Fn as Decode,
		Registry::From::Fn::Fn as Registry,
		Update::Interpret::Fn::Fn as Interpret,
	},
	State::Struct::AphroditeState,
};

/// Update a background task from a `process(action='poll')` result.
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
