//! The nudge pass: which states speak, and the two-nudge ceiling that keeps
//! poll workers from eating the directive budget.

use crate::{
	PollWorker::{
		Check::Fn::Fn as Check,
		Insert::Fn::Fn as Insert,
		Task::Status::{Enum::Enum as Status, Stored::Struct::Struct as Payload},
	},
	State::{
		Identity::{Command::Struct::Struct as Command, Content::Struct::Struct as Digest},
		Struct::AphroditeState,
	},
};

#[test]
fn Running() {
	let mut State = AphroditeState::default();
	State.Turn.Counter = 5;
	Insert(&mut State, "t1".into(), "terminal".into(), "cargo build".into(), 1);

	Check(&mut State);
	assert!(!State.Directive.Ephemeral.is_empty(), "should push at least one nudge");
	let Nudge = State.Directive.Ephemeral.last().unwrap();
	assert!(
		Nudge.inline.as_deref().unwrap().contains("cargo build"),
		"nudge should mention the command"
	);
}

#[test]
fn Completion() {
	let mut State = AphroditeState::default();
	State.Turn.Counter = 6;
	Insert(&mut State, "t1".into(), "terminal".into(), "cargo build".into(), 1);
	State.PollWorker.Tasks[0].Status = Status::Done {
		Output:Payload { Hash:Digest::from("hash123"), Preview:"[build:0E 2W 450L]".into(), Size:450 },
	};

	Check(&mut State);
	let Nudge = State.Directive.Ephemeral.last().unwrap();
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
fn Cap() {
	let mut State = AphroditeState::default();
	State.Turn.Counter = 5;
	// Insert 4 running tasks - only 2 should get nudges.
	(0..4).for_each(|i| {
		Insert(
			&mut State,
			Command::from(format!("t{}", i)),
			"terminal".into(),
			format!("cmd{}", i),
			i,
		);
	});

	// Count nudges pushed BEFORE check_bg_tasks.
	let Before = State.Directive.Ephemeral.len();

	Check(&mut State);

	// At most 2 new nudges should be pushed.
	let NewNudges = State.Directive.Ephemeral.len() - Before;
	assert!(NewNudges <= 2, "must push at most 2 nudges, pushed {}", NewNudges);
}
