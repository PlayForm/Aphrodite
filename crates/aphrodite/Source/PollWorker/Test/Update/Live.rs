//! The live-container progress surface: bytes-so-far, the ETA heuristic, and
//! the instant-marker contract (task hash up front, output hash only at
//! completion, and the two are never the same value).

use crate::{
	PollWorker::{
		Insert::Fn::Fn as Insert,
		Marker::Fn::Fn as Marker,
		Task::Status::{Enum::Enum as Status, Stored::Fn::Fn as Stored},
		Update::Fn::Fn as Update,
	},
	State::{Identity::Command::Struct::Struct as Command, Struct::AphroditeState},
};

#[test]
fn Progress() {
	let mut State = AphroditeState::default();
	State.LiveContainer = true;
	State.TurnCounter = 5;
	Insert(&mut State, "t1".into(), "terminal".into(), "make build".into(), 1);

	let Poll1 = serde_json::json!({"output": "x".repeat(500), "exit_code": serde_json::Value::Null}).to_string();
	let Consumed = Update(&mut State, "process", &Poll1);
	assert!(Consumed);
	assert_eq!(
		State.BgTasks[0].LiveBytesArrived, 500,
		"bytes-so-far must track accumulated output"
	);
	assert!(
		State.BgTasks[0].LiveEtaS >= 0.0 && State.BgTasks[0].LiveEtaS.is_finite(),
		"running ETA must be a finite non-negative estimate"
	);

	let Poll2 = serde_json::json!({"output": "y".repeat(1000), "exit_code": serde_json::Value::Null}).to_string();
	Update(&mut State, "process", &Poll2);
	// Accumulated output joins polls with '\n': 500 + 1 + 1000.
	assert_eq!(State.BgTasks[0].LiveBytesArrived, 1501, "progress must grow across polls");
	assert_eq!(State.BgTasks[0].Status, Status::Running);

	// Completion zeroes the ETA; the full-content hash arrives later.
	let Done = serde_json::json!({"output": "done", "exit_code": 0}).to_string();
	Update(&mut State, "process", &Done);
	assert!(matches!(State.BgTasks[0].Status, Status::Done { .. }));
	assert_eq!(State.BgTasks[0].LiveEtaS, 0.0, "completed task -> zero ETA");
	assert!(
		Stored(&State.BgTasks[0].Status).is_some(),
		"full content hash must exist at completion"
	);
}

#[test]
fn Instant() {
	// The marker template registry is process-global and the `Marker` template
	// tests register into it; rendering a marker without this guard races them.
	let _G = crate::Marker::Format::Registration::Guard::Fn::Fn();
	let mut State = AphroditeState::default();
	State.LiveContainer = true;
	State.TurnCounter = 3;
	let TaskID = Command::from(headroom_core::ccr::compute_key(b"cargo build"));
	Insert(&mut State, TaskID.clone(), "terminal".into(), "cargo build".into(), 1);

	// Instant marker: the task hash is available immediately, typed poll.
	let Marker = Marker(&TaskID, "cargo build", 3);
	assert!(Marker.contains(TaskID.as_str()), "instant marker must embed the task hash");
	assert!(Marker.contains("|poll|"), "instant marker must be typed poll");

	// Progress: partial arrival while still running - no full-content
	// hash yet.
	let Partial = serde_json::json!({"output": "compiling...", "exit_code": serde_json::Value::Null}).to_string();
	assert!(Update(&mut State, "process", &Partial));
	assert_eq!(State.BgTasks[0].Status, Status::Running);
	assert!(Stored(&State.BgTasks[0].Status).is_none(), "no full-content hash while running");

	// Full content arrives later: only at completion does the output
	// hash appear, and it differs from the instant task hash.
	let Done = serde_json::json!({"output": "Finished\n", "exit_code": 0}).to_string();
	assert!(Update(&mut State, "process", &Done));
	assert!(matches!(State.BgTasks[0].Status, Status::Done { .. }));
	let OutHash = Stored(&State.BgTasks[0].Status).expect("output hash at completion");
	assert_ne!(
		OutHash.Hash.as_str(),
		TaskID.as_str(),
		"full-content hash arrives later and differs from the task hash"
	);
	assert_eq!(State.BgTasks[0].LiveEtaS, 0.0);
}
