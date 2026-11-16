//! Render the poll-worker block the agent reads at the top of a turn.
//!
//! The header is emitted from the active list itself, so an empty set produces an
//! empty string by construction rather than by a branch: no active task means no
//! section, which is what keeps a quiet turn byte-identical to a turn before the
//! feature existed.

use crate::{
	PollWorker::{
		Registry::{Iterate::Fn::Fn as Iterate, ReadFrom::Fn::Fn as ReadFrom},
		Status::Detail::Fn::Fn as Detail,
		Task::{Status::Enum::Enum as Status, Struct::Struct as Record},
	},
	State::Struct::AphroditeState,
};

/// Render bg task status lines for injection into `BuildTurnContext`.
/// Returns empty string if no active tasks.
///
/// The empty string is the contract, not a fallback: a turn with no running task
/// renders no section at all, so a quiet turn stays byte-identical to a turn from
/// before the poll worker existed.
pub fn Fn(State:&AphroditeState) -> String {
	let Tasks = ReadFrom(&State.BgTasks);
	// The feature flag is part of the renderer's contract, not a caller's: a disabled
	// poll worker renders no section at all, so "the feature is off" and "nothing is
	// running" produce the same empty string and cannot drift apart.
	let Active:Vec<&Record> = Iterate(&Tasks)
		.filter(|Task| State.PollWorkerEnabled && Task.Status == Status::Running)
		.collect();

	Active
		.first()
		.map(|_| format!("[poll workers] {} active task(s):", Active.len()))
		.into_iter()
		.chain(Active.iter().take(3).flat_map(|Task| Detail(State.LiveContainer, Task)))
		.collect::<Vec<String>>()
		.join("\n")
}
