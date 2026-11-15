//! Admission, deduplication and eviction of tracked tasks.

use crate::{
	PollWorker::{
		Insert::Fn::InsertBgTask,
		Task::{Const::MaximumBgTasks, Status::Enum::BgStatus},
	},
	State::AphroditeState,
};

#[test]
fn TestInsertBgTaskDedup() {
	let mut State = AphroditeState::default();
	InsertBgTask(&mut State, "task1".into(), "terminal".into(), "cargo build".into(), 1);
	assert_eq!(State.BgTasks.len(), 1);

	// Same ID, still running - should dedup.
	InsertBgTask(&mut State, "task1".into(), "terminal".into(), "cargo build".into(), 2);
	assert_eq!(State.BgTasks.len(), 1);
}

#[test]
fn TestInsertBgTaskEvictionAtCapacity() {
	let mut State = AphroditeState::default();
	for i in 0..MaximumBgTasks + 1 {
		InsertBgTask(&mut State, format!("task{i}"), "terminal".into(), format!("cmd{i}"), i);
	}
	assert_eq!(State.BgTasks.len(), MaximumBgTasks, "must not exceed MAX_BG_TASKS");
}

#[test]
fn TestBgTaskDedupPreventsDuplicateRunningTasks() {
	let mut State = AphroditeState::default();
	let ID = headroom_core::ccr::compute_key(b"cargo build");

	InsertBgTask(&mut State, ID.clone(), "terminal".into(), "cargo build".into(), 1);
	InsertBgTask(&mut State, ID.clone(), "terminal".into(), "cargo build".into(), 2);
	InsertBgTask(&mut State, ID, "terminal".into(), "cargo build".into(), 3);

	assert_eq!(State.BgTasks.len(), 1, "duplicate running tasks must be deduplicated");
}

#[test]
fn TestBgTaskEvictionPrefersStaleThenDoneThenOldestRunning() {
	let mut State = AphroditeState::default();
	// Fill to capacity (4) with running tasks.
	for i in 0..4 {
		InsertBgTask(&mut State, format!("t{}", i), "terminal".into(), format!("cmd{}", i), i);
	}
	// Mark t1 as stale.
	State.BgTasks[1].Status = BgStatus::Stale;

	// Insert a 5th - should evict the stale t1.
	InsertBgTask(&mut State, "t5".into(), "terminal".into(), "cmd5".into(), 5);
	assert_eq!(State.BgTasks.len(), 4);
	let IDs:Vec<&str> = State.BgTasks.iter().map(|t| t.ID.as_str()).collect();
	assert!(!IDs.contains(&"t1"), "stale task t1 must be evicted first");
	assert!(IDs.contains(&"t5"), "new task t5 must be present");

	// Mark t0 as Done, then insert t6 - evicts t0.
	State.BgTasks.iter_mut().find(|t| t.ID == "t0").unwrap().Status = BgStatus::Done { ExitCode:0 };
	InsertBgTask(&mut State, "t6".into(), "terminal".into(), "cmd6".into(), 6);
	assert_eq!(State.BgTasks.len(), 4);
	let IDs:Vec<&str> = State.BgTasks.iter().map(|t| t.ID.as_str()).collect();
	assert!(!IDs.contains(&"t0"), "done task t0 must be evicted second");

	// All running - insert t7, evicts oldest running (t2).
	InsertBgTask(&mut State, "t7".into(), "terminal".into(), "cmd7".into(), 7);
	let IDs:Vec<&str> = State.BgTasks.iter().map(|t| t.ID.as_str()).collect();
	assert!(!IDs.contains(&"t2"), "oldest running task t2 must be evicted last");
}
