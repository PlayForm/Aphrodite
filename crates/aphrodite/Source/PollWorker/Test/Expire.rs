//! Staleness marking: the turn gap that marks a task abandoned, and the
//! polls that must not trip it.

use crate::{
	PollWorker::{Expire::Fn::ExpireStaleTasks, Insert::Fn::InsertBgTask, Task::Status::Enum::BgStatus},
	State::AphroditeState,
};

#[test]
fn TestExpireStaleTasks() {
	let mut State = AphroditeState::default();
	State.TurnCounter = 20;
	InsertBgTask(
		&mut State,
		"stale".into(),
		"terminal".into(),
		"old cmd".into(),
		10, // started at turn 10, gap=10 < 16 so retain won't drop it
	);
	// Never polled - gap from turn 20 to last_poll 10 = 10 > STALE_TURN_AGE(8).
	State.BgTasks[0].LastPollTurn = 10;

	ExpireStaleTasks(&mut State);
	assert_eq!(
		State.BgTasks[0].Status,
		BgStatus::Stale,
		"unpolled task at turn 20 should be stale (last poll: turn 10, gap=10 > 8)"
	);
}

#[test]
fn TestExpireStaleDoesNotTouchRecentlyPolledTasks() {
	let mut State = AphroditeState::default();
	State.TurnCounter = 20;
	InsertBgTask(&mut State, "active".into(), "terminal".into(), "recent-cmd".into(), 15);
	State.BgTasks[0].LastPollTurn = 19; // polled just one turn ago

	ExpireStaleTasks(&mut State);
	assert_eq!(
		State.BgTasks[0].Status,
		BgStatus::Running,
		"recently polled task must not be marked stale"
	);
}
