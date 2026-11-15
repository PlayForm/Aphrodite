//! The runtime action vocabulary, the ephemeral-nudge lifecycle, and the
//! manual-mutation latch.
//!
//! `field_reassign_with_default` triggers on the verbatim
//! `Default::default()` + `s.turn_counter = N` setup pattern inherited from
//! the monolithic test file - pre-existing in the original code, preserved
//! as-is for the frozen test contract.
#![allow(clippy::field_reassign_with_default)]

use crate::Directive::{Action::Fn as HandleAction, Model::Struct as Directive};

// ── 01-F8: `HandleAction` is the single implementation both
// `AphroditeDirective` and `AphroditeDispatch`'s `"directive"` arm now
// delegate to - cover all six actions plus an unknown one directly. ──
#[test]
fn TestHandleActionAllActionsAndUnknown() {
	let mut State = crate::State::Struct::AphroditeState::default();
	State
		.Directives
		.insert("focus".into(), Directive { name:"focus".into(), Content:"stay focused".into() });
	State
		.Directives
		.insert("lazy".into(), Directive { name:"lazy".into(), Content:"defer work".into() });

	let r = HandleAction(&mut State, "list", "");
	assert_eq!(r["available"], serde_json::json!(["focus", "lazy"]));
	assert_eq!(r["active"], serde_json::json!([]));

	let r = HandleAction(&mut State, "swap", "focus");
	assert_eq!(r["swapped"], "focus");
	assert_eq!(State.ActiveDirectives, vec!["focus".to_string()]);

	let r = HandleAction(&mut State, "swap", "nonexistent");
	assert!(r["error"].as_str().unwrap().contains("unknown directive"));

	let r = HandleAction(&mut State, "remove", "focus");
	assert_eq!(r["active"], serde_json::json!([]));

	// ── "load" activates a known directive and returns {loaded, active};
	// loading an already-active directive is idempotent and still
	// reports success (no error). ──
	let r = HandleAction(&mut State, "load", "lazy");
	assert_eq!(r["loaded"], "lazy");
	assert_eq!(r["active"], serde_json::json!(["lazy"]));
	let r = HandleAction(&mut State, "load", "lazy");
	assert_eq!(r["loaded"], "lazy", "re-loading must be idempotent");
	// "load" errors on an unknown directive (unlike "add", which is silent).
	let r = HandleAction(&mut State, "load", "ghost");
	assert!(r["error"].as_str().unwrap().contains("unknown directive"));

	let r = HandleAction(&mut State, "add", "focus");
	assert_eq!(r["active"], serde_json::json!(["lazy", "focus"]));

	let r = HandleAction(&mut State, "reset", "");
	assert_eq!(r["active"], serde_json::json!([]));
	assert!(State.ActiveDirectives.is_empty());

	let r = HandleAction(&mut State, "bogus", "");
	assert!(r["error"].as_str().unwrap().contains("unknown action"));
}

// ── P3/T9: a one-shot nudge (ttl=1) pushed during turn N renders in turn
// N+1's context exactly once, then is purged by post_llm_call so it never
// shows again. ──
#[test]
fn TestOneShotNudgeRendersOnceThenPurged() {
	let mut s = crate::State::Struct::AphroditeState::default();
	s.TurnCounter = 5;
	crate::Flow::Nudge::Push::Fn::PushNudge(&mut s, "step back and restate the goal", 1);

	// Simulate the turn advancing (post_llm_call runs next_turn then purge):
	// during turn 5 the nudge is live; render at turn 6 must include it.
	s.TurnCounter = 6;
	let Ctx6 = crate::Flow::TurnContext::Fn::BuildTurnContext(&mut s, None);
	assert!(Ctx6.contains("[nudge:"), "one-shot nudge must render at turn 6: {Ctx6}");
	assert!(Ctx6.contains("step back and restate the goal"));

	// post_llm_call advances to turn 7 and purges expired nudges.
	crate::Hook::LLM::Post::Fn::Fn(&mut s);
	assert_eq!(s.TurnCounter, 7);
	let Ctx7 = crate::Flow::TurnContext::Fn::BuildTurnContext(&mut s, None);
	assert!(!Ctx7.contains("[nudge:"), "expired nudge must be gone at turn 7: {Ctx7}");
}

// ── P3/T10: at most 4 ephemeral entries are stored; a 5th drops the
// oldest. ──
#[test]
fn TestNudgeCapDropsOldest() {
	let mut s = crate::State::Struct::AphroditeState::default();

	(0..5).for_each(|i| {
		crate::Flow::Nudge::Push::Fn::PushNudge(&mut s, &format!("nudge number {i}"), 10);
	});
	assert_eq!(s.EphemeralDirectives.len(), 4, "cap must hold at 4 stored entries");
	assert!(
		!s.EphemeralDirectives
			.iter()
			.any(|e| e.inline.as_deref() == Some("nudge number 0")),
		"the oldest nudge must have been dropped"
	);
	assert!(
		s.EphemeralDirectives
			.iter()
			.any(|e| e.inline.as_deref() == Some("nudge number 4")),
		"the newest nudge must survive"
	);
}

// ── P3/T10: `list` surfaces ephemeral entries; `reset` clears them and the
// manual latch. ──
#[test]
fn TestListShowsEphemeralAndResetClearsThem() {
	let mut s = crate::State::Struct::AphroditeState::default();
	s.TurnCounter = 3;
	crate::Flow::Nudge::Push::Fn::PushNudge(&mut s, "watch out", 2);
	s.ManualDirectiveTurn = Some(3);

	let Listed = HandleAction(&mut s, "list", "");
	let Eph = Listed["ephemeral"].as_array().expect("ephemeral array");
	assert_eq!(Eph.len(), 1);
	assert_eq!(Eph[0]["inline"], "watch out");
	assert_eq!(Eph[0]["expires_after_turn"], 5);

	HandleAction(&mut s, "reset", "");
	assert!(s.EphemeralDirectives.is_empty(), "reset must clear ephemeral nudges");
	assert!(s.ManualDirectiveTurn.is_none(), "reset must clear the manual latch");
}

// ── P1/T3: any manual mutation latches `ManualDirectiveTurn` to the
// current turn (P6 override latch). ──
#[test]
fn TestManualMutationSetsManualDirectiveTurn() {
	let mut s = crate::State::Struct::AphroditeState::default();
	s.TurnCounter = 12;
	s.Directives
		.insert("focus".into(), Directive { name:"focus".into(), Content:"stay focused".into() });
	HandleAction(&mut s, "swap", "focus");
	assert_eq!(s.ManualDirectiveTurn, Some(12), "a manual swap must latch the turn");

	// A failed swap (unknown directive) must NOT latch.
	s.ManualDirectiveTurn = None;
	HandleAction(&mut s, "swap", "nope");
	assert!(s.ManualDirectiveTurn.is_none(), "a failed swap must not latch");
}
