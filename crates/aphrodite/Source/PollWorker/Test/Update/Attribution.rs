//! Which task a poll applies to (audit D6).
//!
//! One poll result finishes one task. The pre-rewrite ingestion parsed a single
//! exit code and applied it to every running task, so a second, still-running build
//! was reported done with the other task's code.

use crate::{
	PollWorker::{
		Insert::Fn::Fn as Insert,
		Task::Status::{Enum::Enum as Status, Stored::Fn::Fn as Stored},
		Update::Fn::Fn as Update,
	},
	State::{
		Identity::{Command::Struct::Struct as Command, Execution::Struct::Struct as Execution},
		Struct::AphroditeState,
	},
};

fn TwoRunning(State:&mut AphroditeState) {
	Insert(
		State,
		Command::from(headroom_core::ccr::compute_key(b"cargo build")),
		"terminal".into(),
		"cargo build".into(),
		1,
	);
	Insert(
		State,
		Command::from(headroom_core::ccr::compute_key(b"npm install")),
		"terminal".into(),
		"npm install".into(),
		2,
	);
}

#[test]
fn One() {
	let mut State = AphroditeState::default();
	State.TurnCounter = 5;
	TwoRunning(&mut State);

	let Poll = serde_json::json!({
		"command": "cargo build",
		"output": "Finished dev profile",
		"exit_code": 0
	})
	.to_string();

	assert!(Update(&mut State, "process", &Poll));

	assert!(
		matches!(State.BgTasks[0].Status, Status::Done { .. }),
		"the polled task completes"
	);
	assert_eq!(State.BgTasks[1].Status, Status::Running, "the other task keeps running");
	assert!(
		Stored(&State.BgTasks[1].Status).is_none(),
		"the other task records no stored output"
	);
	assert_eq!(State.BgTasks[1].PollCount, 0, "the other task was not polled");
	assert!(
		State.BgTasks[1].AccumulatedOutput.AsStr().is_empty(),
		"the other task accumulated nothing"
	);
}

#[test]
fn Untracked() {
	let mut State = AphroditeState::default();
	State.TurnCounter = 5;
	TwoRunning(&mut State);

	let Poll = serde_json::json!({
		"command": "make release",
		"output": "done",
		"exit_code": 0
	})
	.to_string();

	assert!(!Update(&mut State, "process", &Poll), "an untracked process is not consumed");
	assert_eq!(State.BgTasks[0].Status, Status::Running);
	assert_eq!(State.BgTasks[1].Status, Status::Running);
}

#[test]
fn Fallback() {
	let mut State = AphroditeState::default();
	State.TurnCounter = 5;
	TwoRunning(&mut State);

	// A bare text poll names no command and no session - the only attribution it
	// supports is the oldest running task.
	assert!(Update(&mut State, "process", "building...\nexit code: 2\n"));

	assert!(
		matches!(State.BgTasks[0].Status, Status::Failed { ExitCode, .. } if ExitCode.get() == 2),
		"the oldest running task takes the failure"
	);
	assert_eq!(State.BgTasks[1].Status, Status::Running);
}

#[test]
fn Execution() {
	let mut State = AphroditeState::default();
	State.TurnCounter = 5;
	TwoRunning(&mut State);

	// First poll names the process only; it is attributed to the oldest running
	// task and the identity is recorded there.
	Update(
		&mut State,
		"process",
		&serde_json::json!({"session_id": "proc-7", "output": "a", "exit_code": null}).to_string(),
	);
	assert_eq!(
		State.BgTasks[0].Execution.as_ref().map(Execution::as_str),
		Some("proc-7"),
		"the execution identity is recorded on the task it was attributed to"
	);

	// A later poll naming the same process reaches the same task even though the
	// oldest running task is still that one - the identity, not the position, is
	// what matched.
	Update(
		&mut State,
		"process",
		&serde_json::json!({"session_id": "proc-7", "output": "b", "exit_code": 0}).to_string(),
	);
	assert!(matches!(State.BgTasks[0].Status, Status::Done { .. }));
	assert_eq!(State.BgTasks[1].Status, Status::Running);
}
