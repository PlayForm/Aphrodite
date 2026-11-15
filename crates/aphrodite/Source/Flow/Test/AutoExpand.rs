//! Auto-expand guidance: injected only while `state.auto_expand` is on, and
//! rendered through the prompt renderer so a configured template wins.

use super::Support::StateWithPrompt;
use crate::{Flow::BuildTurnContext, State::AphroditeState};

#[test]
fn TestAutoExpandGuidanceInjectedWhenEnabled() {
	let mut s = AphroditeState::default();
	s.AutoExpand = true;
	s.TurnCounter = 5;
	s.FlowBudgetChars = 4000;
	let Context = BuildTurnContext(&mut s, None);
	assert!(
		Context.contains("[auto-expand:") && Context.contains("Tool outputs are auto-expanded"),
		"auto-expand guidance must be injected when enabled: {Context}"
	);
}

#[test]
fn TestAutoExpandGuidanceAbsentWhenDisabled() {
	let mut s = AphroditeState::default(); // auto_expand = false
	s.TurnCounter = 5;
	s.FlowBudgetChars = 4000;
	let Context = BuildTurnContext(&mut s, None);
	assert!(
		!Context.contains("[auto-expand:") && !Context.contains("auto-expanded"),
		"absent auto-expand must inject no guidance: {Context}"
	);
}

#[test]
fn TestAutoExpandGuidanceUsesConfiguredTemplate() {
	let mut s = StateWithPrompt("auto_expand_guidance", "inline {limit} limit");
	s.AutoExpand = true;
	s.TurnCounter = 5;
	s.FlowBudgetChars = 4000;
	let Context = BuildTurnContext(&mut s, None);
	assert!(
		Context.contains("[auto-expand: inline {limit} limit]"),
		"configured template must win over the fallback: {Context}"
	);
}
