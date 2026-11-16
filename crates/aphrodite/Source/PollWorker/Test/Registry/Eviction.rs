//! Eviction order: stale, then finished, then running, oldest first within each.

use crate::{
	PollWorker::{
		Insert::Fn::Fn as Insert,
		Registry::ReadFrom::Fn::Fn as ReadFrom,
		Select::Eviction::Fn::Fn as Evict,
		Task::Status::{Enum::Enum as Status, Stored::Struct::Struct as Payload},
	},
	State::{
		Identity::{Command::Struct::Struct as Command, Content::Struct::Struct as Digest},
		Struct::AphroditeState,
	},
};

fn Finished() -> Payload { Payload { Hash:Digest::from("h"), Preview:"[terminal] out".into(), Size:3 } }

fn Four() -> AphroditeState {
	let mut State = AphroditeState::default();
	(0..4).for_each(|i| {
		Insert(
			&mut State,
			Command::from(format!("t{i}")),
			"terminal".into(),
			format!("cmd{i}"),
			i,
		)
	});
	State
}

#[test]
fn Oldest() {
	let mut State = Four();
	State.PollWorker.Tasks[1].Status = Status::Stale;
	State.PollWorker.Tasks[3].Status = Status::Stale;

	assert_eq!(
		Evict(&ReadFrom(&State.PollWorker.Tasks)),
		Some(1),
		"the older of two stale tasks goes"
	);
}

#[test]
fn Order() {
	let mut State = Four();
	State.PollWorker.Tasks[2].Status = Status::Done { Output:Finished() };
	State.PollWorker.Tasks[3].Status = Status::Stale;

	assert_eq!(Evict(&ReadFrom(&State.PollWorker.Tasks)), Some(3), "stale outranks finished");

	State.PollWorker.Tasks[3].Status = Status::Running;
	assert_eq!(Evict(&ReadFrom(&State.PollWorker.Tasks)), Some(2), "finished outranks running");
}

#[test]
fn Last() {
	let State = Four();

	assert_eq!(
		Evict(&ReadFrom(&State.PollWorker.Tasks)),
		Some(0),
		"with nothing expendable the front of the queue goes"
	);
}

#[test]
fn Empty() {
	let State = AphroditeState::default();

	assert_eq!(Evict(&ReadFrom(&State.PollWorker.Tasks)), None);
}
