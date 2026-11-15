//! The live-container progress surface: bytes-so-far, the ETA heuristic, and
//! the instant-marker contract (task hash up front, output hash only at
//! completion, and the two are never the same value).

use crate::{
	PollWorker::{
		Insert::Fn::InsertBgTask,
		Marker::Fn::CreatePollMarker,
		Task::Status::Enum::BgStatus,
		Update::Fn::UpdateFromPoll,
	},
	State::Struct::AphroditeState,
};

#[test]
fn TestPollLiveProgressBytesSoFarAndEta() {
	let mut State = AphroditeState::default();
	State.LiveContainer = true;
	State.TurnCounter = 5;
	InsertBgTask(&mut State, "t1".into(), "terminal".into(), "make build".into(), 1);

	let Poll1 = serde_json::json!({"output": "x".repeat(500), "exit_code": serde_json::Value::Null}).to_string();
	let Consumed = UpdateFromPoll(&mut State, "process", &Poll1);
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
	UpdateFromPoll(&mut State, "process", &Poll2);
	// Accumulated output joins polls with '\n': 500 + 1 + 1000.
	assert_eq!(State.BgTasks[0].LiveBytesArrived, 1501, "progress must grow across polls");
	assert_eq!(State.BgTasks[0].Status, BgStatus::Running);

	// Completion zeroes the ETA; the full-content hash arrives later.
	let Done = serde_json::json!({"output": "done", "exit_code": 0}).to_string();
	UpdateFromPoll(&mut State, "process", &Done);
	assert_eq!(State.BgTasks[0].Status, BgStatus::Done { ExitCode:0 });
	assert_eq!(State.BgTasks[0].LiveEtaS, 0.0, "completed task -> zero ETA");
	assert!(
		State.BgTasks[0].OutputHash.is_some(),
		"full content hash must exist at completion"
	);
}

#[test]
fn TestPollInstantMarkerHashAvailableBeforeContent() {
	// The marker template registry is process-global and the `Marker` template
	// tests register into it; rendering a marker without this guard races them.
	let _G = crate::Marker::Format::Const::MarkerTemplatesTestGuard();
	let mut State = AphroditeState::default();
	State.LiveContainer = true;
	State.TurnCounter = 3;
	let TaskID = headroom_core::ccr::compute_key(b"cargo build");
	InsertBgTask(&mut State, TaskID.clone(), "terminal".into(), "cargo build".into(), 1);

	// Instant marker: the task hash is available immediately, typed poll.
	let Marker = CreatePollMarker(&TaskID, "cargo build", 3);
	assert!(Marker.contains(&TaskID), "instant marker must embed the task hash");
	assert!(Marker.contains("|poll|"), "instant marker must be typed poll");

	// Progress: partial arrival while still running - no full-content
	// hash yet.
	let Partial = serde_json::json!({"output": "compiling...", "exit_code": serde_json::Value::Null}).to_string();
	assert!(UpdateFromPoll(&mut State, "process", &Partial));
	assert_eq!(State.BgTasks[0].Status, BgStatus::Running);
	assert!(State.BgTasks[0].OutputHash.is_none(), "no full-content hash while running");

	// Full content arrives later: only at completion does the output
	// hash appear, and it differs from the instant task hash.
	let Done = serde_json::json!({"output": "Finished\n", "exit_code": 0}).to_string();
	assert!(UpdateFromPoll(&mut State, "process", &Done));
	assert_eq!(State.BgTasks[0].Status, BgStatus::Done { ExitCode:0 });
	let OutHash = State.BgTasks[0].OutputHash.as_ref().expect("output hash at completion");
	assert_ne!(
		OutHash, &TaskID,
		"full-content hash arrives later and differs from the task hash"
	);
	assert_eq!(State.BgTasks[0].LiveEtaS, 0.0);
}
