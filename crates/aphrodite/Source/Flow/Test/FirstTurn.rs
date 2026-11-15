//! The one-shot first-turn orientation: rendered on turn 0 only, and only
//! when `SessionInject` is non-empty.

use crate::Flow::TurnContext::Fn::BuildTurnContext;
use crate::Flow::Prompt::Const::ShippedSessionInject;
use crate::State::Struct::AphroditeState;

// ── First-turn session injection: rendered only on turn 0, dropped after ──
#[test]
fn TestFirstTurnInjectionRendersOnTurnZero() {
	let mut s = AphroditeState::default();
	s.SessionInject = ShippedSessionInject.to_string();
	s.TurnCounter = 0;
	let Context = BuildTurnContext(&mut s, None);
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
fn TestFirstTurnInjectionSuppressedAfterTurnZero() {
	let mut s = AphroditeState::default();
	s.SessionInject = ShippedSessionInject.to_string();
	s.TurnCounter = 1;
	let Context = BuildTurnContext(&mut s, None);
	assert!(
		!Context.contains("[aphrodite: first-turn orientation]"),
		"session inject must NOT appear after turn 0: {Context}"
	);
}

#[test]
fn TestFirstTurnInjectionDisabledWhenEmpty() {
	let mut s = AphroditeState::default();
	s.SessionInject = String::new();
	s.TurnCounter = 0;
	let Context = BuildTurnContext(&mut s, None);
	assert!(
		!Context.contains("[aphrodite: first-turn orientation]"),
		"empty session_inject must produce no injection: {Context}"
	);
}

#[test]
fn TestFirstTurnInjectionReplacesVersionPlaceholder() {
	let mut s = AphroditeState::default();
	s.SessionInject = "aphrodite v{VERSION} ready".to_string();
	s.TurnCounter = 0;
	let Context = BuildTurnContext(&mut s, None);
	assert!(
		Context.contains(&format!("aphrodite v{}", env!("CARGO_PKG_VERSION"))),
		"{{VERSION}} must be replaced: {Context}"
	);
	assert!(!Context.contains("{VERSION}"), "unreplaced {{VERSION}} placeholder: {Context}");
}
