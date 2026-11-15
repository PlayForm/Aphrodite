//! The `RetrieveGuidance` knob: minimal (default) injects nothing, standard
//! and verbose inject progressively more, and an unknown value warns and
//! degrades to standard rather than injecting nothing.

use crate::{Flow::BuildTurnContext, State::AphroditeState};

#[test]
fn TestRetrieveGuidanceMinimalInjectsNothing() {
	let mut s = AphroditeState::default();
	s.RetrieveGuidance = "minimal".into();
	s.TurnCounter = 5;
	let Context = BuildTurnContext(&mut s, None);
	assert!(
		!Context.contains("[retrieval:"),
		"minimal must not inject retrieval guidance: {Context}"
	);
}

#[test]
fn TestRetrieveGuidanceDefaultInjectsNothing() {
	let mut s = AphroditeState::default();
	s.TurnCounter = 5;
	let Context = BuildTurnContext(&mut s, None);
	assert!(
		!Context.contains("[retrieval:"),
		"absent knob (default) must not inject: {Context}"
	);
}

#[test]
fn TestRetrieveGuidanceStandardInjectsLine() {
	let mut s = AphroditeState::default();
	s.RetrieveGuidance = "standard".into();
	s.TurnCounter = 5;
	let Context = BuildTurnContext(&mut s, None);
	assert!(
		Context.contains("[retrieval: CCR markers compress large tool output."),
		"standard must inject one guidance line: {Context}"
	);
	assert!(!Context.contains("Never re-read"), "standard must stay single-line: {Context}");
}

#[test]
fn TestRetrieveGuidanceVerboseInjectsExtended() {
	let mut s = AphroditeState::default();
	s.RetrieveGuidance = "verbose".into();
	s.TurnCounter = 5;
	let Context = BuildTurnContext(&mut s, None);
	assert!(
		Context.contains("[retrieval:") && Context.contains("Never re-read a file"),
		"verbose must inject the extended guidance: {Context}"
	);
}

#[test]
fn TestRetrieveGuidanceUnknownValueDegradesToStandard() {
	let mut s = AphroditeState::default();
	s.RetrieveGuidance = "bogus".into();
	s.TurnCounter = 5;
	let Context = BuildTurnContext(&mut s, None);
	assert!(
		Context.contains("[retrieval: CCR markers compress large tool output."),
		"unknown value must warn + degrade to standard: {Context}"
	);
}
