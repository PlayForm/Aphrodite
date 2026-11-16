//! One retention decision per task, at the boundaries the old two-pass rule used.

use crate::{
	PollWorker::{
		Expire::{Const::Lifetime, Decide::Fn::Fn as Decide, Decision::Enum::Enum as Decision},
		Task::{
			Const::Age,
			New::Fn::Fn as New,
			Status::{Enum::Enum as Status, Stored::Struct::Struct as Payload},
		},
	},
	State::Identity::Content::Struct::Struct as Digest,
};

fn Stored() -> Payload { Payload { Hash:Digest::from("h"), Preview:"[terminal] out".into(), Size:3 } }

fn Running(StartedTurn:usize, LastPollTurn:usize) -> crate::PollWorker::Task::Struct::Struct {
	let mut Task = New("t".into(), "terminal".into(), "cargo build".into(), StartedTurn, 0);
	Task.LastPollTurn = LastPollTurn;
	Task
}

#[test]
fn Boundary() {
	let Task = Running(0, 0);
	assert_eq!(Decide(&Task, Age), Decision::Keep, "exactly Age turns is not stale");
	assert_eq!(Decide(&Task, Age + 1), Decision::MarkStale, "one turn past it is");
}

#[test]
fn Stale() {
	let Task = Running(0, 0);
	assert_eq!(
		Decide(&Task, Lifetime),
		Decision::MarkStale,
		"at the lifetime it is only marked"
	);
	assert_eq!(Decide(&Task, Lifetime + 1), Decision::Remove, "past it, it is dropped");
}

#[test]
fn Same() {
	// The old pass marked stale first and then retained, so a task that went stale in
	// this call could also be removed in this call. One decision reproduces that.
	let Task = Running(0, 0);
	let Turn = Age.max(Lifetime) + 1;

	assert_eq!(Decide(&Task, Turn), Decision::Remove);
}

#[test]
fn Old() {
	let Task = Running(0, 0);
	let Turn = Lifetime + 1;

	// Unpolled and young enough to be stale-marked, so it is kept as a record - but a
	// task still being polled is never dropped however old it is.
	let mut Polled = Running(0, 0);
	Polled.LastPollTurn = Turn;

	assert_eq!(Decide(&Polled, Turn), Decision::Keep);
	assert_eq!(Decide(&Task, Turn), Decision::Remove);
}

#[test]
fn Finished() {
	let mut Task = Running(0, 0);
	Task.Status = Status::Done { Output:Stored() };

	assert_eq!(Decide(&Task, Lifetime), Decision::Keep);
	assert_eq!(Decide(&Task, Lifetime + 1), Decision::Remove);
}

#[test]
fn Backwards() {
	let Task = Running(10, 10);

	assert_eq!(
		Decide(&Task, 0),
		Decision::Keep,
		"saturating_sub keeps a rewound counter from expiring anything"
	);
}
