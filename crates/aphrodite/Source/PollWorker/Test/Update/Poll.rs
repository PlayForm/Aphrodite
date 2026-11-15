//! Exit-code detection (JSON wrapper and text fallback), output accumulation
//! across polls, and the 256KB accumulation cap.

use crate::PollWorker::Insert::Fn::InsertBgTask;
use crate::PollWorker::Task::Status::Enum::BgStatus;
use crate::PollWorker::Update::Fn::UpdateFromPoll;
use crate::State::Struct::AphroditeState;

#[test]
fn TestUpdateFromPollExitCodeZero() {
	let mut State = AphroditeState::default();
	State.TurnCounter = 5;
	InsertBgTask(
		&mut State,
		headroom_core::ccr::compute_key(b"cargo build"),
		"terminal".into(),
		"cargo build".into(),
		1,
	);

	let PollJSON = serde_json::json!({
		"output": "Compiling...\nFinished\nexit code: 0",
		"exit_code": 0
	})
	.to_string();

	let Consumed = UpdateFromPoll(&mut State, "process", &PollJSON);
	assert!(Consumed, "poll result should be consumed");
	assert_eq!(State.BgTasks[0].Status, BgStatus::Done { ExitCode:0 });
	assert!(State.BgTasks[0].OutputHash.is_some());
	assert!(State.BgTasks[0].OutputPreview.is_some());
}

#[test]
fn TestUpdateFromPollExitCodeNonzero() {
	let mut State = AphroditeState::default();
	State.TurnCounter = 5;
	InsertBgTask(
		&mut State,
		headroom_core::ccr::compute_key(b"cargo build"),
		"terminal".into(),
		"cargo build".into(),
		1,
	);

	let PollJSON = serde_json::json!({
		"output": "error: compilation failed\nexit code: 1",
		"exit_code": 1
	})
	.to_string();

	let Consumed = UpdateFromPoll(&mut State, "process", &PollJSON);
	assert!(Consumed);
	assert_eq!(State.BgTasks[0].Status, BgStatus::Failed { ExitCode:1 });
}

#[test]
fn TestUpdateFromPollNoExitCodeStillRunning() {
	let mut State = AphroditeState::default();
	State.TurnCounter = 5;
	InsertBgTask(
		&mut State,
		headroom_core::ccr::compute_key(b"cargo build"),
		"terminal".into(),
		"cargo build".into(),
		1,
	);

	let PollJSON = serde_json::json!({
		"output": "still compiling...",
		"exit_code": null
	})
	.to_string();

	let Consumed = UpdateFromPoll(&mut State, "process", &PollJSON);
	assert!(Consumed);
	assert_eq!(State.BgTasks[0].Status, BgStatus::Running);
	assert_eq!(State.BgTasks[0].PollCount, 1);
	assert!(State.BgTasks[0].AccumulatedOutput.contains("still compiling"));
}

// ── Repetition & edge-case tests ──────────────────────────

#[test]
fn TestMultipleRapidPollsAccumulateOutput() {
	let mut State = AphroditeState::default();
	State.TurnCounter = 5;
	InsertBgTask(&mut State, "t1".into(), "terminal".into(), "long-running".into(), 1);

	// Rapid successive polls - each should accumulate.
	for i in 0..5 {
		let PollJSON = serde_json::json!({
			"output": format!("line {}", i),
			"exit_code": serde_json::Value::Null,
		})
		.to_string();
		let Consumed = UpdateFromPoll(&mut State, "process", &PollJSON);
		assert!(Consumed, "poll {} should be consumed", i);
	}

	assert_eq!(State.BgTasks[0].PollCount, 5);
	assert!(State.BgTasks[0].AccumulatedOutput.contains("line 0"));
	assert!(State.BgTasks[0].AccumulatedOutput.contains("line 4"));
	assert_eq!(State.BgTasks[0].Status, BgStatus::Running);
}

#[test]
fn TestPollWithExitCodeCompletesTask() {
	let mut State = AphroditeState::default();
	State.TurnCounter = 10;
	InsertBgTask(&mut State, "t1".into(), "terminal".into(), "npm install".into(), 1);

	// First: running poll
	let PollRunning = serde_json::json!({
		"output": "installing packages...",
		"exit_code": serde_json::Value::Null,
	})
	.to_string();
	UpdateFromPoll(&mut State, "process", &PollRunning);
	assert_eq!(State.BgTasks[0].Status, BgStatus::Running);

	// Second: completion poll
	let PollDone = serde_json::json!({
		"output": "added 142 packages in 5s",
		"exit_code": 0,
	})
	.to_string();
	UpdateFromPoll(&mut State, "process", &PollDone);
	assert_eq!(State.BgTasks[0].Status, BgStatus::Done { ExitCode:0 });
	assert!(State.BgTasks[0].OutputHash.is_some());
}

#[test]
fn TestPollWithTextExitCodeCompletesTask() {
	let mut State = AphroditeState::default();
	State.TurnCounter = 5;
	InsertBgTask(&mut State, "t1".into(), "terminal".into(), "make build".into(), 1);

	// No JSON wrapper - just raw output with "exit code: 2" in text.
	let Consumed = UpdateFromPoll(&mut State, "process", "building...\nfailed!\nexit code: 2\n");
	assert!(Consumed);
	assert_eq!(State.BgTasks[0].Status, BgStatus::Failed { ExitCode:2 });
}

#[test]
fn TestAccumulatedOutputCappedAt256kb() {
	let mut State = AphroditeState::default();
	State.TurnCounter = 5;
	InsertBgTask(&mut State, "t1".into(), "terminal".into(), "very-verbose".into(), 1);

	// Push 300KB of output across multiple polls.
	let Chunk = "x".repeat(65536); // 64KB
	for _ in 0..5 {
		let Poll = serde_json::json!({
			"output": Chunk.clone(),
			"exit_code": serde_json::Value::Null,
		})
		.to_string();
		UpdateFromPoll(&mut State, "process", &Poll);
	}

	// Should be capped at ~256KB (256 * 1024).
	assert!(
		State.BgTasks[0].AccumulatedOutput.len() <= 256 * 1024 + 100,
		"accumulated output must be capped near 256KB, got {}",
		State.BgTasks[0].AccumulatedOutput.len()
	);
}
