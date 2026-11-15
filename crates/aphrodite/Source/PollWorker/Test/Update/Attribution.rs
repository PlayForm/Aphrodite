//! Which task a poll applies to (audit D6).
//!
//! One poll result finishes one task. The pre-rewrite ingestion parsed a single
//! exit code and applied it to every running task, so a second, still-running build
//! was reported done with the other task's code.

use crate::{
	PollWorker::{Insert::Fn::InsertBgTask, Task::Status::Enum::BgStatus, Update::Fn::UpdateFromPoll},
	State::Struct::AphroditeState,
};

fn TwoRunning(State:&mut AphroditeState) {
	InsertBgTask(
		State,
		headroom_core::ccr::compute_key(b"cargo build"),
		"terminal".into(),
		"cargo build".into(),
		1,
	);
	InsertBgTask(
		State,
		headroom_core::ccr::compute_key(b"npm install"),
		"terminal".into(),
		"npm install".into(),
		2,
	);
}

#[test]
fn TestPollNamingOneCommandFinishesOnlyThatTask() {
	let mut State = AphroditeState::default();
	State.TurnCounter = 5;
	TwoRunning(&mut State);

	let Poll = serde_json::json!({
		"command": "cargo build",
		"output": "Finished dev profile",
		"exit_code": 0
	})
	.to_string();

	assert!(UpdateFromPoll(&mut State, "process", &Poll));

	assert_eq!(
		State.BgTasks[0].Status,
		BgStatus::Done { ExitCode:0 },
		"the polled task completes"
	);
	assert_eq!(State.BgTasks[1].Status, BgStatus::Running, "the other task keeps running");
	assert!(State.BgTasks[1].OutputHash.is_none(), "the other task records no output hash");
	assert_eq!(State.BgTasks[1].PollCount, 0, "the other task was not polled");
	assert!(
		State.BgTasks[1].AccumulatedOutput.AsStr().is_empty(),
		"the other task accumulated nothing"
	);
}

#[test]
fn TestPollNamingAnUntrackedCommandAppliesToNothing() {
	let mut State = AphroditeState::default();
	State.TurnCounter = 5;
	TwoRunning(&mut State);

	let Poll = serde_json::json!({
		"command": "make release",
		"output": "done",
		"exit_code": 0
	})
	.to_string();

	assert!(
		!UpdateFromPoll(&mut State, "process", &Poll),
		"an untracked process is not consumed"
	);
	assert_eq!(State.BgTasks[0].Status, BgStatus::Running);
	assert_eq!(State.BgTasks[1].Status, BgStatus::Running);
}

#[test]
fn TestPollWithoutIdentityFallsBackToOldestRunning() {
	let mut State = AphroditeState::default();
	State.TurnCounter = 5;
	TwoRunning(&mut State);

	// A bare text poll names no command and no session - the only attribution it
	// supports is the oldest running task.
	assert!(UpdateFromPoll(&mut State, "process", "building...\nexit code: 2\n"));

	assert_eq!(State.BgTasks[0].Status, BgStatus::Failed { ExitCode:2 });
	assert_eq!(State.BgTasks[1].Status, BgStatus::Running);
}

#[test]
fn TestPollExecutionIdentitySticksToTheTaskItNamed() {
	let mut State = AphroditeState::default();
	State.TurnCounter = 5;
	TwoRunning(&mut State);

	// First poll names the process only; it is attributed to the oldest running
	// task and the identity is recorded there.
	UpdateFromPoll(
		&mut State,
		"process",
		&serde_json::json!({"session_id": "proc-7", "output": "a", "exit_code": null}).to_string(),
	);
	assert_eq!(State.BgTasks[0].ExecutionID.as_deref(), Some("proc-7"));

	// A later poll naming the same process reaches the same task even though the
	// oldest running task is still that one - the identity, not the position, is
	// what matched.
	UpdateFromPoll(
		&mut State,
		"process",
		&serde_json::json!({"session_id": "proc-7", "output": "b", "exit_code": 0}).to_string(),
	);
	assert_eq!(State.BgTasks[0].Status, BgStatus::Done { ExitCode:0 });
	assert_eq!(State.BgTasks[1].Status, BgStatus::Running);
}
