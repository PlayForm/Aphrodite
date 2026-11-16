//! Capacity is a non-zero type, and admission never exceeds it.

use crate::{
	PollWorker::{Insert::Fn::Fn as Insert, Registry::Capacity::Struct::Capacity, Task::Const::Maximum},
	State::{Identity::Command::Struct::Struct as Command, Struct::AphroditeState},
};

#[test]
fn Policy() {
	assert_eq!(Capacity::Policy.Get(), Maximum);
	assert!(Capacity::Policy.Get() > 0, "a zero capacity is not a representable state");
}

#[test]
fn Admission() {
	let mut State = AphroditeState::default();
	(0..Maximum + 3).for_each(|i| {
		Insert(
			&mut State,
			Command::from(format!("task{i}")),
			"terminal".into(),
			format!("cmd{i}"),
			i,
		);
	});

	assert_eq!(State.PollWorker.Tasks.len(), Maximum);
}
