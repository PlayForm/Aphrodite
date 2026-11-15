//! The accumulator's bound (audit D4).
//!
//! The cap used to be a length test taken before an append it never measured, so one
//! large poll could exceed the stated bound by its whole payload. It is now a property
//! of the accumulator: the append is cut to the room that is left, on a char boundary,
//! and the live-container byte count can never report more than the bound.

use crate::{
	PollWorker::{
		Insert::Fn::InsertBgTask,
		Output::Const::MaximumAccumulatedBytes,
		Task::Status::Enum::BgStatus,
		Update::Fn::UpdateFromPoll,
	},
	State::Struct::AphroditeState,
};

fn OneTask() -> AphroditeState {
	let mut State = AphroditeState::default();
	State.LiveContainer = true;
	State.TurnCounter = 5;
	InsertBgTask(&mut State, "t1".into(), "terminal".into(), "very-verbose".into(), 1);
	State
}

#[test]
fn TestOneOversizedPollIsCutToTheBound() {
	let mut State = OneTask();

	// A single 400 KB poll - larger than the cap in one append.
	let Poll = serde_json::json!({"output": "x".repeat(400 * 1024), "exit_code": serde_json::Value::Null}).to_string();
	assert!(UpdateFromPoll(&mut State, "process", &Poll));

	assert_eq!(
		State.BgTasks[0].AccumulatedOutput.Len(),
		MaximumAccumulatedBytes,
		"the cap holds after one oversized poll"
	);
	assert_eq!(
		State.BgTasks[0].LiveBytesArrived, MaximumAccumulatedBytes,
		"bytes arrived never exceeds the cap"
	);
	assert_eq!(State.BgTasks[0].Status, BgStatus::Running);
}

#[test]
fn TestAccumulationAcrossPollsStopsAtTheBound() {
	let mut State = OneTask();
	let Chunk = "x".repeat(64 * 1024);

	(0..5).for_each(|_| {
		let Poll = serde_json::json!({"output": Chunk.clone(), "exit_code": serde_json::Value::Null}).to_string();
		UpdateFromPoll(&mut State, "process", &Poll);
	});

	assert_eq!(State.BgTasks[0].AccumulatedOutput.Len(), MaximumAccumulatedBytes);
	assert_eq!(State.BgTasks[0].LiveBytesArrived, MaximumAccumulatedBytes);
}

#[test]
fn TestTruncationKeepsAValidUtf8Prefix() {
	let mut State = OneTask();

	// A multi-byte character straddling the bound must not be split: the kept prefix
	// is cut back to the last whole character.
	let Payload = "é".repeat(MaximumAccumulatedBytes);
	let Poll = serde_json::json!({"output": Payload, "exit_code": serde_json::Value::Null}).to_string();
	assert!(UpdateFromPoll(&mut State, "process", &Poll));

	let Kept = State.BgTasks[0].AccumulatedOutput.AsStr();
	assert_eq!(
		Kept.len(),
		MaximumAccumulatedBytes,
		"two bytes per char, so an even bound is met exactly"
	);
	assert!(
		Kept.chars().all(|Char| Char == 'é'),
		"no replacement character, no split code point"
	);
}

#[test]
fn TestCompletedTaskStoresTheKeptPrefix() {
	let mut State = OneTask();

	let Poll = serde_json::json!({"output": "x".repeat(400 * 1024), "exit_code": 0}).to_string();
	UpdateFromPoll(&mut State, "process", &Poll);

	assert_eq!(State.BgTasks[0].Status, BgStatus::Done { ExitCode:0 });
	assert_eq!(
		State.BgTasks[0].LiveBytesArrived, MaximumAccumulatedBytes,
		"the final byte count is the stored size"
	);
	assert_eq!(State.BgTasks[0].LiveEtaS, 0.0);
	assert_eq!(
		State.BgTasks[0].AccumulatedOutput.Len(),
		0,
		"the buffer was moved into the store"
	);
}
