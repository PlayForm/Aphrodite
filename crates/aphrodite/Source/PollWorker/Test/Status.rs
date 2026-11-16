//! The turn-context renderer: empty state, mixed state, and the
//! live-container line's on/off contract.

use crate::{
	PollWorker::{
		Insert::Fn::Fn as Insert,
		Status::Fn::Fn as Render,
		Task::Status::{Enum::Enum as Status, Stored::Struct::Struct as Payload},
	},
	State::{Identity::Content::Struct::Struct as Digest, Struct::AphroditeState},
};

fn Finished() -> Payload { Payload { Hash:Digest::from("h"), Preview:"[terminal] out".into(), Size:3 } }

#[test]
fn Flag() {
	// A disabled poll worker renders no section even with a running task: the flag is
	// the renderer's contract, so the two ways of having nothing to say agree.
	let mut State = AphroditeState::default();
	Insert(&mut State, "t1".into(), "terminal".into(), "cargo build".into(), 1);
	State.PollWorkerEnabled = false;

	assert_eq!(Render(&State), "");
}

#[test]
fn Empty() {
	let State = AphroditeState::default();
	assert_eq!(Render(&State), "");
}

#[test]
fn Tasks() {
	let mut State = AphroditeState::default();
	Insert(&mut State, "task1".into(), "terminal".into(), "cargo build".into(), 5);
	Insert(&mut State, "task2".into(), "terminal".into(), "npm install".into(), 6);
	let Rendered = Render(&State);
	assert!(Rendered.contains("[poll workers]"));
	assert!(Rendered.contains("cargo build"));
	assert!(Rendered.contains("npm install"));
	assert!(Rendered.contains("not polled yet"));
}

#[test]
fn Mixed() {
	let mut State = AphroditeState::default();
	Insert(&mut State, "r1".into(), "terminal".into(), "running1".into(), 1);
	Insert(&mut State, "r2".into(), "terminal".into(), "running2".into(), 2);
	// Mark one as done.
	State.BgTasks[1].Status = Status::Done { Output:Finished() };
	Insert(&mut State, "r3".into(), "terminal".into(), "running3".into(), 3);

	let Rendered = Render(&State);
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
fn Live() {
	// Off (default): byte-identical to the pre-port renderer.
	let mut State = AphroditeState::default();
	assert!(!State.LiveContainer, "default state must have live_container off");
	Insert(&mut State, "t1".into(), "terminal".into(), "cargo build".into(), 1);
	let Expected = "[poll workers] 1 active task(s):\n  · `cargo build` - running since turn 1 (not polled yet)";
	assert_eq!(Render(&State), Expected, "off-path render must be byte-identical");

	// On: a live-container line with byte progress appears.
	State.LiveContainer = true;
	let Rendered = Render(&State);
	assert!(
		Rendered.contains("[live container]"),
		"live mode must add a container line: {Rendered}"
	);
	assert!(
		Rendered.contains("bytes arrived so far"),
		"progress must be surfaced: {Rendered}"
	);
}
