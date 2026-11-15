//! Capacity is a non-zero type, and admission never exceeds it.

use crate::{
	PollWorker::{Insert::Fn::InsertBgTask, Registry::Capacity::Struct::Capacity, Task::Const::MaximumBgTasks},
	State::Struct::AphroditeState,
};

#[test]
fn TestCapacityIsThePolicyBoundAndNonZero() {
	assert_eq!(Capacity::Policy.Get(), MaximumBgTasks);
	assert!(Capacity::Policy.Get() > 0, "a zero capacity is not a representable state");
}

#[test]
fn TestAdmissionNeverExceedsCapacity() {
	let mut State = AphroditeState::default();
	(0..MaximumBgTasks + 3).for_each(|i| {
		InsertBgTask(&mut State, format!("task{i}"), "terminal".into(), format!("cmd{i}"), i);
	});

	assert_eq!(State.BgTasks.len(), MaximumBgTasks);
}
