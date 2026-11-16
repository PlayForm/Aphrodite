//! Staleness marking: the turn gap that marks a task abandoned, and the
//! polls that must not trip it.

use crate::{
	PollWorker::{Expire::Fn::Fn as Expire, Insert::Fn::Fn as Insert, Task::Status::Enum::Enum as Status},
	State::Struct::AphroditeState,
};

#[test]
fn All() {
	let mut State = AphroditeState::default();
	State.Turn.Counter = 20;
	Insert(
		&mut State,
		"stale".into(),
		"terminal".into(),
		"old cmd".into(),
		10, // started at turn 10, gap=10 < 16 so retain won't drop it
	);
	// Never polled - gap from turn 20 to last_poll 10 = 10 > STALE_TURN_AGE(8).
	State.PollWorker.Tasks[0].LastPollTurn = 10;

	Expire(&mut State);
	assert_eq!(
		State.PollWorker.Tasks[0].Status,
		Status::Stale,
		"unpolled task at turn 20 should be stale (last poll: turn 10, gap=10 > 8)"
	);
}

#[test]
fn Recent() {
	let mut State = AphroditeState::default();
	State.Turn.Counter = 20;
	Insert(&mut State, "active".into(), "terminal".into(), "recent-cmd".into(), 15);
	State.PollWorker.Tasks[0].LastPollTurn = 19; // polled just one turn ago

	Expire(&mut State);
	assert_eq!(
		State.PollWorker.Tasks[0].Status,
		Status::Running,
		"recently polled task must not be marked stale"
	);
}
