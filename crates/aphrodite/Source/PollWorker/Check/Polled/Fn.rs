//! How to say whether a running task has been polled.
//!
//! Its own leaf because the clause is the one part of the running sentence that
//! depends on the task's history rather than its identity, and the two wordings
//! are what tell the agent "you have never looked" apart from "you looked, here is
//! when".

use crate::PollWorker::Task::Struct::Struct as Record;

/// The parenthetical a running-task nudge carries.
pub(crate) fn Fn(Task:&Record) -> String {
	match Task.PollCount {
		0 => String::from(" (not polled yet)"),
		Count => format!(" (polled {Count}×, last turn {})", Task.LastPollTurn),
	}
}
