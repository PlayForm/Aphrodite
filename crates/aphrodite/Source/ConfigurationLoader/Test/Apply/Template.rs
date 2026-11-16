//! `[templates]` publication and first-turn session injection.

use std::collections::HashMap;

use super::super::Guard::{GlobalsGuards, GuardedApplyCompression};
use crate::ConfigurationLoader::Configuration::Struct::Struct as Configuration;

// ── Pre-Rust port: `ApplyGlobals` publishes `[templates]` +
// `[previews] model_family` into the process-global renderer so the
// dylib path renders `[templates.preview.<family>.<ctype>]` format
// strings, together with the marker grammar and the preview cap. ──
#[test]
fn TestApplyGlobalsRegistersTemplatesAndFamily() {
	// One publication writes three process-wide registries, so this test holds all
	// three guards for its WHOLE span: `cargo test` runs module bodies concurrently,
	// and a sibling test that reset the marker registration between the publish and
	// the assertions would make this test read another test's state.
	let _Guards = GlobalsGuards();

	let cfg = Configuration {
		raw:"[templates.preview.code_first]\ndiff = \"{type} {ln}L\"\n[templates.marker]\nformat = \
		     \"{preview}\"\nhint = \"H\"\n[prompts]\nccr_marker_hint = true\n[previews]\nmodel_family = \
		     \"code_first\"\n"
			.parse()
			.unwrap(),
		Overrides:HashMap::new(),
		ParseFailure:None,
	};
	let mut State = crate::State::Struct::AphroditeState::default();
	cfg.ApplyCompression(&mut State);
	cfg.ApplyGlobals();
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
	// The same publication carries the marker grammar, which used to be registered by
	// the `[templates]` section applier - a second, partial publication point.
	assert!(crate::Marker::Format::Registration::With::Fn::Fn(|Registered| {
		Registered.Hint.is_some()
	}));
	assert!(crate::Marker::Format::Registration::With::Fn::Fn(|Registered| {
		matches!(
			&Registered.Grammar,
			crate::Marker::Format::Grammar::Enum::Enum::Template(Format) if Format == "{preview}"
		)
	}));

	// Restore globals for later tests: an empty document publishes the defaults.
	Configuration::default().ApplyGlobals();
	assert!(crate::Marker::Format::Registration::With::Fn::Fn(|Registered| {
		matches!(&Registered.Grammar, crate::Marker::Format::Grammar::Enum::Enum::Canonical)
	}));
}

#[test]
fn TestSessionInjectPrefersTemplatesPromptsOverPrompts() {
	let _G = crate::Marker::Format::Registration::Guard::Fn::Fn();
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
	let mut State = crate::State::Struct::AphroditeState::default();
	cfg.ApplyCompression(&mut State);
	assert_eq!(State.SessionInject, "templates text");

	// No [templates.prompts] => the legacy [prompts] location still works.
	let Cfg2 = Configuration {
		raw:"[prompts]\nsession_inject = \"legacy text\"\n".parse().unwrap(),
		Overrides:HashMap::new(),
		ParseFailure:None,
	};
	let mut State2 = crate::State::Struct::AphroditeState::default();
	Cfg2.ApplyCompression(&mut State2);
	assert_eq!(State2.SessionInject, "legacy text");

	// Neither => shipped constant.
	let mut State3 = crate::State::Struct::AphroditeState::default();
	Configuration::default().ApplyCompression(&mut State3);
	assert_eq!(State3.SessionInject, crate::Flow::Prompt::Const::ShippedSessionInject);
}

#[test]
fn TestSessionInjectEnvironmentOverrideBeatsTemplatesPrompts() {
	let _G = crate::Marker::Format::Registration::Guard::Fn::Fn();
	let mut cfg = Configuration::default();
	cfg.SetOverride("APHRODITE_SESSION_INJECT", "env wins");
	let mut State = crate::State::Struct::AphroditeState::default();
	cfg.ApplyCompression(&mut State);
	assert_eq!(State.SessionInject, "env wins");
}
