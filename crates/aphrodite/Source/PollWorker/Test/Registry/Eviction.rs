//! Eviction order: stale, then finished, then running, oldest first within each.

use crate::{
	PollWorker::{
		Insert::Fn::InsertBgTask,
		Registry::ReadFrom::Fn::Fn as ReadFrom,
		Select::Eviction::Fn::Fn as Evict,
		Task::Status::Enum::BgStatus,
	},
	State::Struct::AphroditeState,
};

fn Four() -> AphroditeState {
	let mut State = AphroditeState::default();
	(0..4).for_each(|i| InsertBgTask(&mut State, format!("t{i}"), "terminal".into(), format!("cmd{i}"), i));
	State
}

#[test]
fn TestOldestStaleIsEvictedFirst() {
	let mut State = Four();
	State.BgTasks[1].Status = BgStatus::Stale;
	State.BgTasks[3].Status = BgStatus::Stale;

	assert_eq!(Evict(&ReadFrom(&State.BgTasks)), Some(1), "the older of two stale tasks goes");
}

#[test]
fn TestFinishedBeatsRunningAndLosesToStale() {
	let mut State = Four();
	State.BgTasks[2].Status = BgStatus::Done { ExitCode:0 };
	State.BgTasks[3].Status = BgStatus::Stale;

	assert_eq!(Evict(&ReadFrom(&State.BgTasks)), Some(3), "stale outranks finished");

	State.BgTasks[3].Status = BgStatus::Running;
	assert_eq!(Evict(&ReadFrom(&State.BgTasks)), Some(2), "finished outranks running");
}

#[test]
fn TestOldestRunningIsTheLastResort() {
	let State = Four();

	assert_eq!(
		Evict(&ReadFrom(&State.BgTasks)),
		Some(0),
		"with nothing expendable the front of the queue goes"
	);
}

#[test]
fn TestEmptyRegistryHasNoVictim() {
	let State = AphroditeState::default();

	assert_eq!(Evict(&ReadFrom(&State.BgTasks)), None);
}
