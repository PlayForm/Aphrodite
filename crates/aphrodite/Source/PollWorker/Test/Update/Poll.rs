//! Exit-code detection (JSON wrapper and text fallback), output accumulation
//! across polls, and the 256KB accumulation cap.

use crate::{
	PollWorker::{
		Insert::Fn::Fn as Insert,
		Task::Status::{Enum::Enum as Status, Stored::Fn::Fn as Stored},
		Update::Fn::Fn as Update,
	},
	State::Struct::AphroditeState,
};

#[test]
fn Success() {
	let mut State = AphroditeState::default();
	State.Turn.Counter = 5;
	Insert(
		&mut State,
		headroom_core::ccr::compute_key(b"cargo build").into(),
		"terminal".into(),
		"cargo build".into(),
		1,
	);

	let PollJSON = serde_json::json!({
		"output": "Compiling...\nFinished\nexit code: 0",
		"exit_code": 0
	})
	.to_string();

	let Consumed = Update(&mut State, "process", &PollJSON);
	assert!(Consumed, "poll result should be consumed");
	assert!(matches!(State.PollWorker.Tasks[0].Status, Status::Done { .. }));
	assert!(
		Stored(&State.PollWorker.Tasks[0].Status).is_some(),
		"the completion records its stored output"
	);
}

#[test]
fn Failure() {
	let mut State = AphroditeState::default();
	State.Turn.Counter = 5;
	Insert(
		&mut State,
		headroom_core::ccr::compute_key(b"cargo build").into(),
		"terminal".into(),
		"cargo build".into(),
		1,
	);

	let PollJSON = serde_json::json!({
		"output": "error: compilation failed\nexit code: 1",
		"exit_code": 1
	})
	.to_string();

	let Consumed = Update(&mut State, "process", &PollJSON);
	assert!(Consumed);
	assert!(matches!(State.PollWorker.Tasks[0].Status, Status::Failed { ExitCode, .. } if ExitCode.get() == 1));
}

#[test]
fn Running() {
	let mut State = AphroditeState::default();
	State.Turn.Counter = 5;
	Insert(
		&mut State,
		headroom_core::ccr::compute_key(b"cargo build").into(),
		"terminal".into(),
		"cargo build".into(),
		1,
	);

	let PollJSON = serde_json::json!({
		"output": "still compiling...",
		"exit_code": null
	})
	.to_string();

	let Consumed = Update(&mut State, "process", &PollJSON);
	assert!(Consumed);
	assert_eq!(State.PollWorker.Tasks[0].Status, Status::Running);
	assert_eq!(State.PollWorker.Tasks[0].PollCount, 1);
	assert!(State.PollWorker.Tasks[0].AccumulatedOutput.AsStr().contains("still compiling"));
}

// ── Repetition & edge-case tests ──────────────────────────

#[test]
fn Multiple() {
	let mut State = AphroditeState::default();
	State.Turn.Counter = 5;
	Insert(&mut State, "t1".into(), "terminal".into(), "long-running".into(), 1);

	// Rapid successive polls - each should accumulate.
	(0..5).for_each(|i| {
		let PollJSON = serde_json::json!({
			"output": format!("line {}", i),
			"exit_code": serde_json::Value::Null,
		})
		.to_string();
		let Consumed = Update(&mut State, "process", &PollJSON);
		assert!(Consumed, "poll {} should be consumed", i);
	});

	assert_eq!(State.PollWorker.Tasks[0].PollCount, 5);
	assert!(State.PollWorker.Tasks[0].AccumulatedOutput.AsStr().contains("line 0"));
	assert!(State.PollWorker.Tasks[0].AccumulatedOutput.AsStr().contains("line 4"));
	assert_eq!(State.PollWorker.Tasks[0].Status, Status::Running);
}

#[test]
fn Exit() {
	let mut State = AphroditeState::default();
	State.Turn.Counter = 10;
	Insert(&mut State, "t1".into(), "terminal".into(), "npm install".into(), 1);

	// First: running poll
	let PollRunning = serde_json::json!({
		"output": "installing packages...",
		"exit_code": serde_json::Value::Null,
	})
	.to_string();
	Update(&mut State, "process", &PollRunning);
	assert_eq!(State.PollWorker.Tasks[0].Status, Status::Running);

	// Second: completion poll
	let PollDone = serde_json::json!({
		"output": "added 142 packages in 5s",
		"exit_code": 0,
	})
	.to_string();
	Update(&mut State, "process", &PollDone);
	assert!(matches!(State.PollWorker.Tasks[0].Status, Status::Done { .. }));
	assert!(Stored(&State.PollWorker.Tasks[0].Status).is_some());
}

#[test]
fn Text() {
	let mut State = AphroditeState::default();
	State.Turn.Counter = 5;
	Insert(&mut State, "t1".into(), "terminal".into(), "make build".into(), 1);

	// No JSON wrapper - just raw output with "exit code: 2" in text.
	let Consumed = Update(&mut State, "process", "building...\nfailed!\nexit code: 2\n");
	assert!(Consumed);
	assert!(matches!(State.PollWorker.Tasks[0].Status, Status::Failed { ExitCode, .. } if ExitCode.get() == 2));
}

#[test]
fn Cap() {
	let mut State = AphroditeState::default();
	State.Turn.Counter = 5;
	Insert(&mut State, "t1".into(), "terminal".into(), "very-verbose".into(), 1);

	// Push 300KB of output across multiple polls.
	let Chunk = "x".repeat(65536); // 64KB
	(0..5).for_each(|_| {
		let Poll = serde_json::json!({
			"output": Chunk.clone(),
			"exit_code": serde_json::Value::Null,
		})
		.to_string();
		Update(&mut State, "process", &Poll);
	});

	// Should be capped at ~256KB (256 * 1024).
	assert!(
		State.PollWorker.Tasks[0].AccumulatedOutput.Len() <= 256 * 1024 + 100,
		"accumulated output must be capped near 256KB, got {}",
		State.PollWorker.Tasks[0].AccumulatedOutput.Len()
	);
}
