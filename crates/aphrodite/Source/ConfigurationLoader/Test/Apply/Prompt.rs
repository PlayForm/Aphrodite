//! `[prompts]` knobs - retrieval guidance and catalog intent hints.

use std::collections::HashMap;

use crate::ConfigurationLoader::Configuration::Struct::Struct as Configuration;

// ── Prompt knobs (pre-Rust port): retrieve_guidance / ccr_marker_hint /
// catalog_intent_hints previously parsed with no consumer. ──
//
// The marker grammar these knobs feed is a PROCESS GLOBAL, published by
// `Global`; it is asserted in `Apply/Template.rs` alongside the rest of the
// global publication. This file asserts the state fields only, which is what
// `Startup` is responsible for.
#[test]
fn TestPromptKnobsResolveFromTomlAndDefault() {
	let cfg = Configuration {
		raw:"[prompts]\nretrieve_guidance = \"verbose\"\ncatalog_intent_hints = true\nccr_marker_hint = \
		     true\n\n[templates.marker]\nformat = \"{preview}\"\nhint = \"H\"\n"
			.parse()
			.unwrap(),
		Overrides:HashMap::new(),
		ParseFailure:None,
	};
	let mut State = crate::State::Struct::AphroditeState::default();
	cfg.Startup(&mut State);
	assert_eq!(State.RetrieveGuidance, "verbose");
	assert!(State.CatalogIntentHints);

	// Absent everywhere => defaults (minimal / off).
	let mut State2 = crate::State::Struct::AphroditeState::default();
	Configuration::default().Startup(&mut State2);
	assert_eq!(State2.RetrieveGuidance, "minimal");
	assert!(!State2.CatalogIntentHints);
}

#[test]
fn TestPromptKnobsResolveFromEnvironmentOverride() {
	let mut cfg = Configuration::default();
	cfg.Set("APHRODITE_RETRIEVE_GUIDANCE", "standard");
	cfg.Set("APHRODITE_CATALOG_INTENT_HINTS", "1");
	let mut State = crate::State::Struct::AphroditeState::default();
	cfg.Startup(&mut State);
	assert_eq!(State.RetrieveGuidance, "standard");
	assert!(State.CatalogIntentHints);
}
