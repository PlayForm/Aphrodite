//! One retention decision per task, at the boundaries the old two-pass rule used.

use crate::PollWorker::{
	Expire::{Const::FinishedLifetime, Decide::Fn::Fn as Decide, Decision::Enum::RetentionDecision},
	Task::{Const::StaleTurnAge, New::Fn::Fn as New, Status::Enum::BgStatus},
};

fn Running(StartedTurn:usize, LastPollTurn:usize) -> crate::PollWorker::Task::Struct::BgTask {
	let mut Task = New("t".into(), "terminal".into(), "cargo build".into(), StartedTurn, 0);
	Task.LastPollTurn = LastPollTurn;
	Task
}

#[test]
fn TestRunningTaskAtTheStaleBoundaryIsKept() {
	let Task = Running(0, 0);
	assert_eq!(
		Decide(&Task, StaleTurnAge),
		RetentionDecision::Keep,
		"exactly StaleTurnAge turns is not stale"
	);
	assert_eq!(
		Decide(&Task, StaleTurnAge + 1),
		RetentionDecision::MarkStale,
		"one turn past it is"
	);
}

#[test]
fn TestStaleTaskIsKeptUntilItsLifetimeRunsOut() {
	let Task = Running(0, 0);
	assert_eq!(
		Decide(&Task, FinishedLifetime),
		RetentionDecision::MarkStale,
		"at the lifetime it is only marked"
	);
	assert_eq!(
		Decide(&Task, FinishedLifetime + 1),
		RetentionDecision::Remove,
		"past it, it is dropped"
	);
}

#[test]
fn TestNewlyStaleCanBeRemovedInTheSameCall() {
	// The old pass marked stale first and then retained, so a task that went stale in
	// this call could also be removed in this call. One decision reproduces that.
	let Task = Running(0, 0);
	let Turn = StaleTurnAge.max(FinishedLifetime) + 1;

	assert_eq!(Decide(&Task, Turn), RetentionDecision::Remove);
}

#[test]
fn TestOldRunningTaskIsNeverRemoved() {
	let Task = Running(0, 0);
	let Turn = FinishedLifetime + 1;

	// Unpolled and young enough to be stale-marked, so it is kept as a record - but a
	// task still being polled is never dropped however old it is.
	let mut Polled = Running(0, 0);
	Polled.LastPollTurn = Turn;

	assert_eq!(Decide(&Polled, Turn), RetentionDecision::Keep);
	assert_eq!(Decide(&Task, Turn), RetentionDecision::Remove);
}

#[test]
fn TestFinishedTaskKeepsItsStatusUntilTheLifetimeRunsOut() {
	let mut Task = Running(0, 0);
	Task.Status = BgStatus::Done { ExitCode:0 };

	assert_eq!(Decide(&Task, FinishedLifetime), RetentionDecision::Keep);
	assert_eq!(Decide(&Task, FinishedLifetime + 1), RetentionDecision::Remove);
}

#[test]
fn TestTurnCounterGoingBackwardsYieldsNoExpiry() {
	let Task = Running(10, 10);

	assert_eq!(
		Decide(&Task, 0),
		RetentionDecision::Keep,
		"saturating_sub keeps a rewound counter from expiring anything"
	);
}
