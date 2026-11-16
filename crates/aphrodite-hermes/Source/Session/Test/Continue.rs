//! The keep-watching policy: what the sweep could NOT do is not a reason to stop.
//!
//! This is the regression proof for the seed: "every session's lock timed out"
//! and "the file says stop" used to be the same `false`, so a reload that reached
//! nobody ended the watch. Each case below is one row of that distinction.

use crate::Session::{
	Broadcast::Report,
	Watcher::{Continue, Outcome},
};

/// One applied pass with the given reach, as the policy sees it.
fn Pass(Applied:usize, Skipped:usize, Reload:usize) -> Outcome::Enum::Enum {
	Outcome::Enum::Enum::Applied(Report::Struct::Struct { Applied, Skipped, Reload })
}

// ── Every session wedged: the sweep could not look at any of them, so it learned
// nothing about what the file says. Stopping here is the defect. ──
#[test]
fn probe_a_sweep_that_could_not_look_keeps_watching() {
	assert!(
		Continue::Fn::Fn(&Pass(0, 3, 0)),
		"a sweep whose every session lock was wedged must not stop the watch"
	);
}

// ── No session registered yet: a sweep that ran before the first session was
// inserted has not been told to stop by anyone. ──
#[test]
fn probe_an_empty_registry_keeps_watching() {
	assert!(
		Continue::Fn::Fn(&Pass(0, 0, 0)),
		"a sweep that ran before the first session registered must not stop the watch"
	);
}

// ── One live session still wants a reload: the ordinary keep-watching case. ──
#[test]
fn probe_a_session_that_still_wants_reload_keeps_watching() {
	assert!(
		Continue::Fn::Fn(&Pass(2, 1, 1)),
		"one session still asking for auto-reload keeps the watch alive"
	);
}

// ── Every REACHED session turned auto_reload off in the file: this is the one
// case where stopping is right. ──
#[test]
fn probe_every_reached_session_turning_reload_off_stops() {
	assert!(
		!Continue::Fn::Fn(&Pass(2, 0, 0)),
		"every reached session said stop, so the watch stops"
	);
}

// ── A broken file keeps watching whatever the sweep reached: the next successful
// write is what clears the failure, and there is no write if the watch is gone. ──
#[test]
fn probe_a_parse_failure_keeps_watching() {
	assert!(
		Continue::Fn::Fn(&Outcome::Enum::Enum::Failed(Report::Struct::Struct {
			Applied:0,
			Skipped:2,
			Reload:0
		})),
		"a parse failure must keep watching even when no session could be reached"
	);
}
