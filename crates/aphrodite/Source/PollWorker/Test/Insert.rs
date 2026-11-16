//! Admission, deduplication and eviction of tracked tasks.

use crate::{
	PollWorker::{
		Insert::Fn::Fn as Insert,
		Task::{
			Const::Maximum,
			Status::{Enum::Enum as Status, Stored::Struct::Struct as Payload},
		},
	},
	State::{
		Identity::{Command::Struct::Struct as Command, Content::Struct::Struct as Digest},
		Struct::AphroditeState,
	},
};

fn Finished() -> Payload { Payload { Hash:Digest::from("h"), Preview:"[terminal] out".into(), Size:3 } }

#[test]
fn Dedup() {
	let mut State = AphroditeState::default();
	Insert(&mut State, "task1".into(), "terminal".into(), "cargo build".into(), 1);
	assert_eq!(State.BgTasks.len(), 1);

	// Same ID, still running - should dedup.
	Insert(&mut State, "task1".into(), "terminal".into(), "cargo build".into(), 2);
	assert_eq!(State.BgTasks.len(), 1);
}

#[test]
fn Capacity() {
	let mut State = AphroditeState::default();
	(0..Maximum + 1).for_each(|i| {
		Insert(
			&mut State,
			Command::from(format!("task{i}")),
			"terminal".into(),
			format!("cmd{i}"),
			i,
		);
	});
	assert_eq!(State.BgTasks.len(), Maximum, "must not exceed MAX_BG_TASKS");
}

#[test]
fn Duplicate() {
	let mut State = AphroditeState::default();
	let ID = Command::from(headroom_core::ccr::compute_key(b"cargo build"));

	Insert(&mut State, ID.clone(), "terminal".into(), "cargo build".into(), 1);
	Insert(&mut State, ID.clone(), "terminal".into(), "cargo build".into(), 2);
	Insert(&mut State, ID, "terminal".into(), "cargo build".into(), 3);

	assert_eq!(State.BgTasks.len(), 1, "duplicate running tasks must be deduplicated");
}

#[test]
fn Order() {
	let mut State = AphroditeState::default();
	// Fill to capacity (4) with running tasks.
	(0..4).for_each(|i| {
		Insert(
			&mut State,
			Command::from(format!("t{}", i)),
			"terminal".into(),
			format!("cmd{}", i),
			i,
		);
	});
	// Mark t1 as stale.
	State.BgTasks[1].Status = Status::Stale;

	// Insert a 5th - should evict the stale t1.
	Insert(&mut State, "t5".into(), "terminal".into(), "cmd5".into(), 5);
	assert_eq!(State.BgTasks.len(), 4);
	let IDs:Vec<&str> = State.BgTasks.iter().map(|t| t.ID.as_str()).collect();
	assert!(!IDs.contains(&"t1"), "stale task t1 must be evicted first");
	assert!(IDs.contains(&"t5"), "new task t5 must be present");

	// Mark t0 as Done, then insert t6 - evicts t0.
	State.BgTasks.iter_mut().find(|t| t.ID.as_str() == "t0").unwrap().Status = Status::Done { Output:Finished() };
	Insert(&mut State, "t6".into(), "terminal".into(), "cmd6".into(), 6);
	assert_eq!(State.BgTasks.len(), 4);
	let IDs:Vec<&str> = State.BgTasks.iter().map(|t| t.ID.as_str()).collect();
	assert!(!IDs.contains(&"t0"), "done task t0 must be evicted second");

	// All running - insert t7, evicts oldest running (t2).
	Insert(&mut State, "t7".into(), "terminal".into(), "cmd7".into(), 7);
	let IDs:Vec<&str> = State.BgTasks.iter().map(|t| t.ID.as_str()).collect();
	assert!(!IDs.contains(&"t2"), "oldest running task t2 must be evicted last");
}
