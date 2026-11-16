//! The nudge pass: which states speak, and the two-nudge ceiling that keeps
//! poll workers from eating the directive budget.

use crate::{
	PollWorker::{
		Check::Fn::CheckBgTasks,
		Insert::Fn::InsertBgTask,
		Task::Status::{Enum::BgStatus, Stored::Struct::StoredOutput},
	},
	State::{
		Identity::{Command::Struct::CommandID, Content::Struct::ContentHash},
		Struct::AphroditeState,
	},
};

#[test]
fn TestCheckBgTasksPushesNudgeForRunning() {
	let mut State = AphroditeState::default();
	State.TurnCounter = 5;
	InsertBgTask(&mut State, "t1".into(), "terminal".into(), "cargo build".into(), 1);

	CheckBgTasks(&mut State);
	assert!(!State.EphemeralDirectives.is_empty(), "should push at least one nudge");
	let Nudge = State.EphemeralDirectives.last().unwrap();
	assert!(
		Nudge.inline.as_deref().unwrap().contains("cargo build"),
		"nudge should mention the command"
	);
}

#[test]
fn TestCheckBgTasksPushesCompletionNudge() {
	let mut State = AphroditeState::default();
	State.TurnCounter = 6;
	InsertBgTask(&mut State, "t1".into(), "terminal".into(), "cargo build".into(), 1);
	State.BgTasks[0].Status = BgStatus::Done {
		Output:StoredOutput { Hash:ContentHash::from("hash123"), Preview:"[build:0E 2W 450L]".into(), Size:450 },
	};

	CheckBgTasks(&mut State);
	let Nudge = State.EphemeralDirectives.last().unwrap();
	assert!(
		Nudge.inline.as_deref().unwrap().contains("completed"),
		"completion nudge should say 'completed'"
	);
	assert!(
		Nudge.inline.as_deref().unwrap().contains("hash123"),
		"completion nudge should include hash"
	);
}

#[test]
fn TestCheckBgTasksRespects2NudgeCap() {
	let mut State = AphroditeState::default();
	State.TurnCounter = 5;
	// Insert 4 running tasks - only 2 should get nudges.
	(0..4).for_each(|i| {
		InsertBgTask(
			&mut State,
			CommandID::from(format!("t{}", i)),
			"terminal".into(),
			format!("cmd{}", i),
			i,
		);
	});

	// Count nudges pushed BEFORE check_bg_tasks.
	let Before = State.EphemeralDirectives.len();

	CheckBgTasks(&mut State);

	// At most 2 new nudges should be pushed.
	let NewNudges = State.EphemeralDirectives.len() - Before;
	assert!(NewNudges <= 2, "must push at most 2 nudges, pushed {}", NewNudges);
}
