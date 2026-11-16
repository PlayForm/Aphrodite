//! Auto-expand guidance: injected only while `state.auto_expand` is on, and
//! rendered through the prompt renderer so a configured template wins.

use super::Support::WithPrompt;
use crate::{Flow::TurnContext::Fn::Fn as Assemble, State::Struct::AphroditeState};

#[test]
fn Injected() {
	let mut s = AphroditeState::default();
	s.AutoExpand = true;
	s.TurnCounter = 5;
	s.FlowBudgetChars = 4000;
	let Context = Assemble(&mut s, None);
	assert!(
		Context.contains("[auto-expand:") && Context.contains("Tool outputs are auto-expanded"),
		"auto-expand guidance must be injected when enabled: {Context}"
	);
}

#[test]
fn Absent() {
	let mut s = AphroditeState::default(); // auto_expand = false
	s.TurnCounter = 5;
	s.FlowBudgetChars = 4000;
	let Context = Assemble(&mut s, None);
	assert!(
		!Context.contains("[auto-expand:") && !Context.contains("auto-expanded"),
		"absent auto-expand must inject no guidance: {Context}"
	);
}

#[test]
fn Template() {
	let mut s = WithPrompt("auto_expand_guidance", "inline {limit} limit");
	s.AutoExpand = true;
	s.TurnCounter = 5;
	s.FlowBudgetChars = 4000;
	let Context = Assemble(&mut s, None);
	assert!(
		Context.contains("[auto-expand: inline {limit} limit]"),
		"configured template must win over the fallback: {Context}"
	);
}
