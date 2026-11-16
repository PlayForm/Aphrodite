//! The `RetrieveGuidance` knob: minimal (default) injects nothing, standard
//! and verbose inject progressively more, and an unknown value warns and
//! degrades to standard rather than injecting nothing.

use crate::{Flow::TurnContext::Fn::Fn as Assemble, State::Struct::AphroditeState};

#[test]
fn Minimal() {
	let mut s = AphroditeState::default();
	s.Configuration.RetrieveGuidance = "minimal".into();
	s.Turn.Counter = 5;
	let Context = Assemble(&mut s, None);
	assert!(
		!Context.contains("[retrieval:"),
		"minimal must not inject retrieval guidance: {Context}"
	);
}

#[test]
fn Default() {
	let mut s = AphroditeState::default();
	s.Turn.Counter = 5;
	let Context = Assemble(&mut s, None);
	assert!(
		!Context.contains("[retrieval:"),
		"absent knob (default) must not inject: {Context}"
	);
}

#[test]
fn Standard() {
	let mut s = AphroditeState::default();
	s.Configuration.RetrieveGuidance = "standard".into();
	s.Turn.Counter = 5;
	let Context = Assemble(&mut s, None);
	assert!(
		Context.contains("[retrieval: CCR markers compress large tool output."),
		"standard must inject one guidance line: {Context}"
	);
	assert!(!Context.contains("Never re-read"), "standard must stay single-line: {Context}");
}

#[test]
fn Verbose() {
	let mut s = AphroditeState::default();
	s.Configuration.RetrieveGuidance = "verbose".into();
	s.Turn.Counter = 5;
	let Context = Assemble(&mut s, None);
	assert!(
		Context.contains("[retrieval:") && Context.contains("Never re-read a file"),
		"verbose must inject the extended guidance: {Context}"
	);
}

#[test]
fn Unknown() {
	let mut s = AphroditeState::default();
	s.Configuration.RetrieveGuidance = "bogus".into();
	s.Turn.Counter = 5;
	let Context = Assemble(&mut s, None);
	assert!(
		Context.contains("[retrieval: CCR markers compress large tool output."),
		"unknown value must warn + degrade to standard: {Context}"
	);
}
