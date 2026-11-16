//! The `Applied` variants, and the lifecycle each one moves a task through.

use std::borrow::Cow;

use crate::{
	PollWorker::{
		Apply::{Applied::Enum::Applied, Fn::Fn as Apply},
		Event::Struct::PollEvent,
		Insert::Fn::InsertBgTask,
		Registry::{Complete::Fn::Fn as Complete, From::Fn::Fn as Registry},
		Task::Status::{
			Enum::BgStatus,
			Stored::{Fn::Fn as Stored, Struct::StoredOutput},
		},
		Update::Fn::UpdateFromPoll,
	},
	State::Struct::AphroditeState,
};

/// A decoded poll, with the output borrowed the way `Decode` borrows it.
fn Event<'a>(Command:Option<&str>, ExitCode:Option<i32>, Output:&'a str, ObservedAtUnix:u64) -> PollEvent<'a> {
	PollEvent {
		Command:Command.map(str::to_string),
		ExecutionID:None,
		ExitCode,
		Output:Cow::Borrowed(Output),
		ObservedAtUnix,
	}
}

fn OneRunning() -> AphroditeState {
	let mut State = AphroditeState::default();
	State.TurnCounter = 4;
	InsertBgTask(&mut State, "cargo build".into(), "terminal".into(), "cargo build".into(), 1);

	State
}

#[test]
fn TestAProgressPollOwnsNoOutput() {
	let mut State = OneRunning();
	let Poll = Event(Some("cargo build"), None, "compiling", 100);

	let Applied = Apply(&mut Registry(&mut State.BgTasks), &Poll, State.TurnCounter).expect("the poll names our task");

	match Applied {
		Applied::Progressed(Subject) => {
			assert_eq!(Subject.Index, 0);
			assert_eq!(Subject.ID.as_str(), "cargo build");
			assert_eq!(Subject.Command, "cargo build");
		},
		Applied::Completed(_) => panic!("a poll with no exit code cannot complete a task"),
	}

	// The accumulator grew and the task is still running: a progress poll's whole
	// effect is inside the task.
	assert_eq!(State.BgTasks[0].Status, BgStatus::Running);
	assert_eq!(State.BgTasks[0].AccumulatedOutput.AsStr(), "compiling");
	assert!(
		Stored(&State.BgTasks[0].Status).is_none(),
		"nothing is stored for a progress poll"
	);
}

#[test]
fn TestACompletionPollOwnsTheAccumulatedOutput() {
	let mut State = OneRunning();
	let First = Event(Some("cargo build"), None, "compiling", 100);
	let _ = Apply(&mut Registry(&mut State.BgTasks), &First, State.TurnCounter);

	let Last = Event(Some("cargo build"), Some(0), "finished", 120);
	let Applied = Apply(&mut Registry(&mut State.BgTasks), &Last, State.TurnCounter).expect("the poll names our task");

	let Finished = match Applied {
		Applied::Completed(Finished) => Finished,
		Applied::Progressed(_) => panic!("a poll carrying an exit code completes the task"),
	};

	// The buffer was MOVED out, joined the way two polls join, and the task is left
	// holding an empty accumulator for `Registry::Complete` to finish.
	assert_eq!(Finished.Output, "compiling\nfinished");
	assert_eq!(Finished.ExitCode, 0);
	assert_eq!(Finished.Subject.Index, 0);
	assert_eq!(State.BgTasks[0].AccumulatedOutput.Len(), 0, "the buffer was moved, not copied");
	assert_eq!(State.BgTasks[0].Status, BgStatus::Running, "the status is `Complete`'s to set");
}

#[test]
fn TestAPollForAnUntrackedCommandAppliesToNothing() {
	let mut State = OneRunning();
	let Poll = Event(Some("make release"), Some(0), "done", 100);

	assert!(
		Apply(&mut Registry(&mut State.BgTasks), &Poll, State.TurnCounter).is_none(),
		"a poll for a process we do not track is not ours"
	);
	assert_eq!(State.BgTasks[0].Status, BgStatus::Running);
	assert_eq!(State.BgTasks[0].PollCount, 0, "an unattributed poll does not touch the task");
}

#[test]
fn TestAPollAfterCompletionAppliesToNothing() {
	let mut State = OneRunning();
	let Poll = Event(Some("cargo build"), Some(0), "finished", 100);

	let Applied = Apply(&mut Registry(&mut State.BgTasks), &Poll, State.TurnCounter).expect("first poll completes");
	let Finished = match Applied {
		Applied::Completed(Finished) => Finished,
		Applied::Progressed(_) => panic!("a poll carrying an exit code completes the task"),
	};

	// The production sequence finishes here: `UpdateFromPoll` stores the moved-out
	// output and records the status, so the task stops being a target. The two calls
	// are driven explicitly because "a finished task" only exists after `Complete` -
	// between them the task is still `Running` with an empty accumulator, which is the
	// recovery window `Registry::Complete` documents.
	let Output = StoredOutput {
		Hash:"abc123def456abc123def456abc123def456".into(),
		Preview:String::new(),
		Size:Finished.Output.len(),
	};
	assert!(Complete(&mut Registry(&mut State.BgTasks), 0, Output, Finished.ExitCode).is_some());

	// The task is not running any more, so `Select::Target` cannot hand it out again -
	// a second exit code cannot re-complete a finished task.
	assert!(
		Apply(&mut Registry(&mut State.BgTasks), &Poll, State.TurnCounter).is_none(),
		"a finished task is no longer a target"
	);
}

#[test]
fn TestOnePollFinishesOneTaskAndLeavesTheOthersRunning() {
	// Audit D6, restated at the level of the value: the exit code belongs to the task
	// the poll named, and the other task neither completes nor accumulates.
	let mut State = AphroditeState::default();
	State.TurnCounter = 4;
	InsertBgTask(&mut State, "cargo build".into(), "terminal".into(), "cargo build".into(), 1);
	InsertBgTask(&mut State, "npm install".into(), "terminal".into(), "npm install".into(), 2);

	let Poll = serde_json::json!({ "command": "cargo build", "output": "Finished", "exit_code": 0 }).to_string();
	assert!(UpdateFromPoll(&mut State, "process", &Poll));

	assert!(
		matches!(State.BgTasks[0].Status, BgStatus::Done { .. }),
		"the polled task completes with its own exit code"
	);
	assert_eq!(State.BgTasks[1].Status, BgStatus::Running, "the other task keeps running");
	assert_eq!(State.BgTasks[1].PollCount, 0);
	assert!(Stored(&State.BgTasks[1].Status).is_none());
}
