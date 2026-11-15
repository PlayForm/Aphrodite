//! `[templates]` registration and first-turn session injection.

use std::collections::HashMap;

use super::super::Guard::GuardedApplyCompression;
use crate::ConfigurationLoader::Configuration::Struct::Struct as Configuration;

// ── Pre-Rust port: `ApplyTemplates` registers `[templates]` +
// `[previews] model_family` into the process-global renderer so the
// dylib path renders `[templates.preview.<family>.<ctype>]` format
// strings. ──
#[test]
fn TestApplyTemplatesRegistersTemplatesAndFamily() {
	let _G = crate::Template::Guard::Fn::Fn();
	let cfg = Configuration {
		raw:"[templates.preview.code_first]\ndiff = \"{type} {ln}L\"\n[previews]\nmodel_family = \"code_first\"\n"
			.parse()
			.unwrap(),
		Overrides:HashMap::new(),
		ParseFailure:None,
	};
	let mut State = crate::State::AphroditeState::default();
	GuardedApplyCompression(&cfg, &mut State);
	cfg.ApplyTemplates(&State);
	assert_eq!(
		crate::Template::Family::Get::Fn::Fn().as_deref(),
		Some("code_first"),
		"model_family must register from [previews]"
	);
	assert_eq!(
		crate::Template::Render::Fn::Fn("diff", "a\nb\nc\n").as_deref(),
		Some("diff 3L"),
		"registered [templates.preview.code_first.diff] must render"
	);
	// Restore globals for later tests.
	crate::Template::Set::Fn::Fn(None);
	crate::Template::Family::Set::Fn::Fn(None);
}

#[test]
fn TestSessionInjectPrefersTemplatesPromptsOverPrompts() {
	let _G = crate::Marker::MarkerTemplatesTestGuard();
	// [templates.prompts] session_inject (canonical pre-Rust location) wins
	// over the legacy [prompts] session_inject.
	let cfg = Configuration {
		raw:"[prompts]\nsession_inject = \"legacy [prompts] text\"\n\n[templates.prompts]\nsession_inject = \
		     \"templates text\"\n"
			.parse()
			.unwrap(),
		Overrides:HashMap::new(),
		ParseFailure:None,
	};
	let mut State = crate::State::AphroditeState::default();
	cfg.ApplyCompression(&mut State);
	assert_eq!(State.SessionInject, "templates text");

	// No [templates.prompts] => the legacy [prompts] location still works.
	let Cfg2 = Configuration {
		raw:"[prompts]\nsession_inject = \"legacy text\"\n".parse().unwrap(),
		Overrides:HashMap::new(),
		ParseFailure:None,
	};
	let mut State2 = crate::State::AphroditeState::default();
	Cfg2.ApplyCompression(&mut State2);
	assert_eq!(State2.SessionInject, "legacy text");

	// Neither => shipped constant.
	let mut State3 = crate::State::AphroditeState::default();
	Configuration::default().ApplyCompression(&mut State3);
	assert_eq!(State3.SessionInject, crate::Flow::ShippedSessionInject);
}

#[test]
fn TestSessionInjectEnvironmentOverrideBeatsTemplatesPrompts() {
	let _G = crate::Marker::MarkerTemplatesTestGuard();
	let mut cfg = Configuration::default();
	cfg.SetOverride("APHRODITE_SESSION_INJECT", "env wins");
	let mut State = crate::State::AphroditeState::default();
	cfg.ApplyCompression(&mut State);
	assert_eq!(State.SessionInject, "env wins");
}
