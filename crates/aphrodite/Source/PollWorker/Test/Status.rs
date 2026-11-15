//! The turn-context renderer: empty state, mixed state, and the
//! live-container line's on/off contract.

use crate::{
	PollWorker::{Insert::Fn::InsertBgTask, Status::Fn::RenderBgTaskStatus, Task::Status::Enum::BgStatus},
	State::AphroditeState,
};

#[test]
fn TestRenderBgTaskStatusEmpty() {
	let State = AphroditeState::default();
	assert_eq!(RenderBgTaskStatus(&State), "");
}

#[test]
fn TestRenderBgTaskStatusWithTasks() {
	let mut State = AphroditeState::default();
	InsertBgTask(&mut State, "task1".into(), "terminal".into(), "cargo build".into(), 5);
	InsertBgTask(&mut State, "task2".into(), "terminal".into(), "npm install".into(), 6);
	let Rendered = RenderBgTaskStatus(&State);
	assert!(Rendered.contains("[poll workers]"));
	assert!(Rendered.contains("cargo build"));
	assert!(Rendered.contains("npm install"));
	assert!(Rendered.contains("not polled yet"));
}

#[test]
fn TestRenderBgTaskStatusWithMixedStates() {
	let mut State = AphroditeState::default();
	InsertBgTask(&mut State, "r1".into(), "terminal".into(), "running1".into(), 1);
	InsertBgTask(&mut State, "r2".into(), "terminal".into(), "running2".into(), 2);
	// Mark one as done.
	State.BgTasks[1].Status = BgStatus::Done { ExitCode:0 };
	InsertBgTask(&mut State, "r3".into(), "terminal".into(), "running3".into(), 3);

	let Rendered = RenderBgTaskStatus(&State);
	assert!(Rendered.contains("[poll workers]"));
	assert!(Rendered.contains("2 active task(s)"), "only running tasks count: {}", Rendered);
	// r3 is running, r1 is running = 2 active.
	// r2 is done - not counted.
}

// ── Live containers (pre-Rust `_hooks/live.py` port): polled tasks
// expose progress state (bytes-so-far / ETA) when `state.live_container`
// is on, and the status renderer's live-container line appears only when
// enabled (off-path byte-identical). ──

#[test]
fn TestRenderLiveContainerLineOnlyWhenEnabled() {
	// Off (default): byte-identical to the pre-port renderer.
	let mut State = AphroditeState::default();
	assert!(!State.LiveContainer, "default state must have live_container off");
	InsertBgTask(&mut State, "t1".into(), "terminal".into(), "cargo build".into(), 1);
	let Expected = "[poll workers] 1 active task(s):\n  · `cargo build` - running since turn 1 (not polled yet)";
	assert_eq!(RenderBgTaskStatus(&State), Expected, "off-path render must be byte-identical");

	// On: a live-container line with byte progress appears.
	State.LiveContainer = true;
	let Rendered = RenderBgTaskStatus(&State);
	assert!(
		Rendered.contains("[live container]"),
		"live mode must add a container line: {Rendered}"
	);
	assert!(
		Rendered.contains("bytes arrived so far"),
		"progress must be surfaced: {Rendered}"
	);
}
