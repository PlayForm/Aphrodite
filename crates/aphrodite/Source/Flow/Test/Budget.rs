//! Budget survival: which sections may be dropped, and in what order.
//!
//! The drop order is the contract. Under budget pressure the recall catalog
//! goes first and the directives, nudges, first-turn orientation and
//! engine-offload prompt never go at all - a turn that silently loses its
//! directives is a correctness failure, not a size optimisation.

use super::Support::{AddMarker, StateWithDirective};
use crate::{
	Flow::{Nudge::Push::Fn::PushNudge, Prompt::Const::ShippedSessionInject, TurnContext::Fn::BuildTurnContext},
	State::Struct::AphroditeState,
};

// ── T1: the assembler drops the recall catalog before directives when the
// budget is too small - directives are never dropped. ──
#[test]
fn TestBudgetDropsCatalogBeforeDirectives() {
	let mut s = StateWithDirective("focus", "stay targeted, minimal tool usage");
	// 50 markers so the catalog_summary is large.
	for i in 0..50 {
		AddMarker(&mut s, &format!("hash{i:040}"), i);
	}
	// Budget large enough for the directive block but too small for the
	// recall block to also fit, so the catalog must be dropped first.
	s.FlowBudgetChars = 90;
	let Context = BuildTurnContext(&mut s, None);
	assert!(Context.contains("[directives:"), "directives must survive: {Context}");
	assert!(
		!Context.contains("[recall]"),
		"catalog must be dropped first under budget pressure: {Context}"
	);
}

// ── Fix 11 regression: turn zero with all three always-survive sections
// (session inject + directive + nudge) plus many markers must drop the
// recall catalog while keeping all three alive. The old inverted
// always_survive (== 1 here) would have dropped the nudge and directive
// too. ──
#[test]
fn TestBudgetKeepsAllAlwaysSurviveSectionsOnTurnZero() {
	let mut s = StateWithDirective("focus", "stay targeted, minimal tool usage");
	s.SessionInject = ShippedSessionInject.to_string();
	s.TurnCounter = 0;
	PushNudge(&mut s, "check marker sizes before retrieving", 1);
	// 50 markers so the recall catalog is large and must drop.
	for i in 0..50 {
		AddMarker(&mut s, &format!("hash{i:040}"), i);
	}
	// Budget well below the three always-survive sections combined, so
	// only the recall catalog may drop.
	s.FlowBudgetChars = 600;
	let Context = BuildTurnContext(&mut s, None);
	assert!(
		Context.contains("[aphrodite: first-turn orientation]"),
		"session inject must survive: {Context}"
	);
	assert!(Context.contains("[directives:"), "directives must survive: {Context}");
	assert!(Context.contains("[nudge:"), "nudge must survive: {Context}");
	assert!(
		!Context.contains("[recall]"),
		"catalog must be the only section dropped: {Context}"
	);
}

// ── T2: the retrieve-hint boilerplate is no longer emitted per turn
// (04-F2 fix). Tool schemas in the system prompt already teach retrieval. ──
#[test]
fn TestRetrieveHintNotInPerTurnContext() {
	let mut s = AphroditeState::default();
	AddMarker(&mut s, &"a".repeat(40), 0);
	s.FlowBudgetChars = 4000;
	let Context = BuildTurnContext(&mut s, None);
	assert_eq!(
		Context.matches("retrieve: aphrodite_retrieve").count(),
		0,
		"retrieve hint must NOT appear in per-turn context (04-F2): {Context}"
	);
}

#[test]
fn TestEmptyStateYieldsEmptyContext() {
	let mut s = AphroditeState::default();
	assert_eq!(BuildTurnContext(&mut s, None), "");
}
