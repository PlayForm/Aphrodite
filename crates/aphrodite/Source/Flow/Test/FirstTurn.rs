//! The one-shot first-turn orientation: rendered on turn 0 only, and only
//! when `SessionInject` is non-empty.

use crate::{
	Flow::{Prompt::Const::Shipped, TurnContext::Fn::Fn as Assemble},
	State::Struct::AphroditeState,
};

// ── First-turn session injection: rendered only on turn 0, dropped after ──
#[test]
fn Zero() {
	let mut s = AphroditeState::default();
	s.Configuration.SessionInject = Shipped.to_string();
	s.Turn.Counter = 0;
	let Context = Assemble(&mut s, None);
	assert!(
		Context.contains("[aphrodite: first-turn orientation]"),
		"session inject must appear on turn 0: {Context}"
	);
	assert!(
		Context.contains("<<<CCR:hash|type|size>>>"),
		"session inject must contain CCR marker example: {Context}"
	);
}

#[test]
fn After() {
	let mut s = AphroditeState::default();
	s.Configuration.SessionInject = Shipped.to_string();
	s.Turn.Counter = 1;
	let Context = Assemble(&mut s, None);
	assert!(
		!Context.contains("[aphrodite: first-turn orientation]"),
		"session inject must NOT appear after turn 0: {Context}"
	);
}

#[test]
fn Disabled() {
	let mut s = AphroditeState::default();
	s.Configuration.SessionInject = String::new();
	s.Turn.Counter = 0;
	let Context = Assemble(&mut s, None);
	assert!(
		!Context.contains("[aphrodite: first-turn orientation]"),
		"empty session_inject must produce no injection: {Context}"
	);
}

#[test]
fn Version() {
	let mut s = AphroditeState::default();
	s.Configuration.SessionInject = "aphrodite v{VERSION} ready".to_string();
	s.Turn.Counter = 0;
	let Context = Assemble(&mut s, None);
	assert!(
		Context.contains(&format!("aphrodite v{}", env!("CARGO_PKG_VERSION"))),
		"{{VERSION}} must be replaced: {Context}"
	);
	assert!(!Context.contains("{VERSION}"), "unreplaced {{VERSION}} placeholder: {Context}");
}
