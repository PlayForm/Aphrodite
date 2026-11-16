//! The accumulator's bound (audit D4).
//!
//! The cap used to be a length test taken before an append it never measured, so one
//! large poll could exceed the stated bound by its whole payload. It is now a property
//! of the accumulator: the append is cut to the room that is left, on a char boundary,
//! and the live-container byte count can never report more than the bound.

use crate::{
	PollWorker::{
		Insert::Fn::Fn as Insert,
		Output::Const::Maximum,
		Task::Status::Enum::Enum as Status,
		Update::Fn::Fn as Update,
	},
	State::Struct::AphroditeState,
};

fn OneTask() -> AphroditeState {
	let mut State = AphroditeState::default();
	State.LiveContainer = true;
	State.TurnCounter = 5;
	Insert(&mut State, "t1".into(), "terminal".into(), "very-verbose".into(), 1);
	State
}

#[test]
fn Oversized() {
	let mut State = OneTask();

	// A single 400 KB poll - larger than the cap in one append.
	let Poll = serde_json::json!({"output": "x".repeat(400 * 1024), "exit_code": serde_json::Value::Null}).to_string();
	assert!(Update(&mut State, "process", &Poll));

	assert_eq!(
		State.BgTasks[0].AccumulatedOutput.Len(),
		Maximum,
		"the cap holds after one oversized poll"
	);
	assert_eq!(
		State.BgTasks[0].LiveBytesArrived, Maximum,
		"bytes arrived never exceeds the cap"
	);
	assert_eq!(State.BgTasks[0].Status, Status::Running);
}

#[test]
fn Bound() {
	let mut State = OneTask();
	let Chunk = "x".repeat(64 * 1024);

	(0..5).for_each(|_| {
		let Poll = serde_json::json!({"output": Chunk.clone(), "exit_code": serde_json::Value::Null}).to_string();
		Update(&mut State, "process", &Poll);
	});

	assert_eq!(State.BgTasks[0].AccumulatedOutput.Len(), Maximum);
	assert_eq!(State.BgTasks[0].LiveBytesArrived, Maximum);
}

#[test]
fn Truncate() {
	let mut State = OneTask();

	// A multi-byte character straddling the bound must not be split: the kept prefix
	// is cut back to the last whole character.
	let Payload = "é".repeat(Maximum);
	let Poll = serde_json::json!({"output": Payload, "exit_code": serde_json::Value::Null}).to_string();
	assert!(Update(&mut State, "process", &Poll));

	let Kept = State.BgTasks[0].AccumulatedOutput.AsStr();
	assert_eq!(Kept.len(), Maximum, "two bytes per char, so an even bound is met exactly");
	assert!(
		Kept.chars().all(|Char| Char == 'é'),
		"no replacement character, no split code point"
	);
}

#[test]
fn Prefix() {
	let mut State = OneTask();

	let Poll = serde_json::json!({"output": "x".repeat(400 * 1024), "exit_code": 0}).to_string();
	Update(&mut State, "process", &Poll);

	assert!(matches!(State.BgTasks[0].Status, Status::Done { .. }));
	assert_eq!(
		State.BgTasks[0].LiveBytesArrived, Maximum,
		"the final byte count is the stored size"
	);
	assert_eq!(State.BgTasks[0].LiveEtaS, 0.0);
	assert_eq!(
		State.BgTasks[0].AccumulatedOutput.Len(),
		0,
		"the buffer was moved into the store"
	);
}
