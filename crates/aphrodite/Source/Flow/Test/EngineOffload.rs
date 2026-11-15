//! The context-engine trigger and its `[engine-offload: ...]` section.
//!
//! While `state.context_engine_enabled` is set, `EngineArchivePrompt`
//! consults `Session::engine_maybe_archive` (the trigger's single call site)
//! and the returned prompt is injected; disabled, or an engine that archived
//! nothing, injects nothing at all. The Some-path injection semantics are
//! exercised on `EngineOffloadSectionFrom`, which is the piece that turns
//! the engine's answer into text.

use crate::Flow::TurnContext::Fn::BuildTurnContext;
use crate::Flow::Engine::Offload::Fn::EngineArchivePrompt;
use crate::Flow::Engine::Offload::Fn::EngineOffloadSectionFrom;
use crate::State::Struct::AphroditeState;

#[test]
fn TestEngineOffloadDisabledNoCallNoInjection() {
	// Disabled: the gate short-circuits inside `EngineArchivePrompt`
	// BEFORE `Session::engine_maybe_archive` (its only call site), so the
	// engine is never consulted and no section can appear.
	let mut s = AphroditeState::default();
	s.ContextEngineEnabled = false;
	s.TurnCounter = 5;
	s.FlowBudgetChars = 4000;
	let Context = BuildTurnContext(&mut s, None);
	assert!(
		!Context.contains("[engine-offload"),
		"disabled engine must inject nothing: {Context}"
	);
	assert_eq!(EngineArchivePrompt(&mut s), None, "gate must yield None while disabled");
}

#[test]
fn TestEngineOffloadPromptInjectedWhenEngineReturnsSome() {
	// Engine returned a prompt => the wrapped section is what the model sees.
	// The full build_turn_context path cannot produce Some yet: the
	// `Session::engine_maybe_archive` stub returns None until Task 3 lands.
	let Section = EngineOffloadSectionFrom(Some("offloaded abc123 keep 5".to_string()));
	assert_eq!(Section, "[engine-offload: offloaded abc123 keep 5]");
	// An empty prompt degrades to no injection, never a bare section.
	assert_eq!(EngineOffloadSectionFrom(Some(String::new())), "");
}

#[test]
fn TestEngineOffloadNothingInjectedWhenEngineReturnsNone() {
	// Engine enabled (default) + stub returning None => no injection.
	let mut s = AphroditeState::default();
	assert_eq!(EngineOffloadSectionFrom(EngineArchivePrompt(&mut s)), "");
	assert_eq!(EngineOffloadSectionFrom(None), "");
}

#[test]
fn TestEngineOffloadDefaultStateUnchanged() {
	// Default state: engine enabled (default) but the pre-landed
	// `EngineMaybeArchive` stub returns None => no archive, no injection -
	// byte-identical current behavior (and the context stays empty).
	let mut s = AphroditeState::default();
	s.TurnCounter = 5;
	s.FlowBudgetChars = 4000;
	let Context = BuildTurnContext(&mut s, None);
	assert!(
		!Context.contains("[engine-offload"),
		"stub engine (None) must inject nothing: {Context}"
	);
	assert_eq!(Context, "", "default state must produce the same empty context as before");
}
