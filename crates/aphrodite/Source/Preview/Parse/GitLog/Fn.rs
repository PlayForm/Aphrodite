//! Drive the record machine over a log and aggregate the records.

use crate::Preview::{
	Input::Struct::Struct as Input,
	Parse::GitLog::{
		Event::Fn::Fn as Event,
		Flush::Fn::Fn as Flush,
		State::Enum::Enum as State,
		Transition::Fn::Fn as Transition,
	},
	Summary::GitLog::Struct::Struct as Summary,
};

/// Parse a `git log` dump into its commit count and its two endpoints.
///
/// The events are fed in input order and each completed record is folded into the
/// summary, which retains only the first and the last - so the memory this holds is two
/// records regardless of how long the log is. The flush at the end closes the last
/// record, which is the only boundary the per-event flush cannot see. It is not a
/// graph: nothing about ancestry is read.
pub(crate) fn Fn(Inp:&Input<'_>) -> Summary {
	let mut State = State::Outside;
	let mut Summary = Summary::new();

	Inp.raw
		.lines()
		.map(Event)
		.filter_map(|Event| Transition(&mut State, &Event))
		.for_each(|Record| Summary.Push(Record));

	Flush(&mut State).into_iter().for_each(|Record| Summary.Push(Record));

	Summary
}
