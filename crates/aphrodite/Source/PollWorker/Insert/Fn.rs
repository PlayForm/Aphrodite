//! Admit a new background task.
//!
//! Deduplication and eviction are the registry's: a running task with the same
//! command identity is not tracked twice, and at capacity the victim is the oldest
//! expendable task (stale, then finished, then running). The eviction is returned
//! by the registry rather than logged here, because "which task made room" is a
//! fact about the set, not about this call.

use crate::{
	PollWorker::{
		Registry::{Admit::Fn::Fn as Admit, From::Fn::Fn as Registry},
		Task::New::Fn::Fn as New,
	},
	State::{Identity::Command::Struct::Struct as Command, Struct::AphroditeState},
};

/// Insert a new background task, evicting the oldest expendable one at capacity.
///
/// `TaskID` is a `Command`, not a string: the caller that mints it is
/// `Should::Command` (a hash of the command line) and the caller that compares it is
/// `Registry::Room` (deduplication). Keeping the type means those two cannot be handed
/// an execution identity or a content hash by mistake.
pub fn Fn(State:&mut AphroditeState, TaskID:Command, Tool:String, command:String, Turn:usize) {
	let ObservedAtUnix = crate::Prefetch::Clock::Fn::Fn();
	let Task = New(TaskID, Tool, command, Turn, ObservedAtUnix);

	Admit(&mut Registry(&mut State.BgTasks), Task);
}
