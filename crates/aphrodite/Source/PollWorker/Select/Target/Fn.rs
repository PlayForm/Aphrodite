//! Which task a decoded poll belongs to.
//!
//! Two identities are consulted before position, because a poll envelope names
//! both: the command line it polled and the process it polled. The command is
//! checked first - it is the task's own identity (`Task::Struct::ID` is its
//! hash), so a match is exact - and the execution identity is recorded on the
//! task the first time a poll names it, which is what makes a second poll of the
//! same process land on the same task.
//!
//! A poll that names a command no tracked task is running is *not* attributed to
//! an unrelated task: it is a poll for a process this session does not track, so
//! the answer is `None` and the result passes through as ordinary output. Only a
//! wire shape that names nothing at all - a bare text result - falls back to the
//! oldest running task, which is the only attribution such a result supports.

use crate::PollWorker::{
	Event::Struct::PollEvent,
	Registry::Reader::Struct::Reader,
	Task::{Status::Enum::BgStatus, Struct::BgTask},
};

/// The one task this poll applies to.
pub(crate) fn Fn(This:&Reader, Event:&PollEvent) -> Option<usize> {
	let Running = |Task:&BgTask| Task.Status == BgStatus::Running;

	let ByCommand = Event
		.Command
		.as_deref()
		.and_then(|Named| This.Tasks.iter().position(|Task| Running(Task) && Task.command == Named));
	let ByExecution = Event.ExecutionID.as_deref().and_then(|Named| {
		This.Tasks
			.iter()
			.position(|Task| Running(Task) && Task.ExecutionID.as_deref() == Some(Named))
	});
	let Oldest = This.Tasks.iter().position(Running);

	match Event.Command.as_deref() {
		// A named command that matches nothing is a poll for an untracked process.
		Some(_) => ByCommand.or(ByExecution),
		None => ByExecution.or(Oldest),
	}
}
