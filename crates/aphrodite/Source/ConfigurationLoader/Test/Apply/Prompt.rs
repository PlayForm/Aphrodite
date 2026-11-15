//! `[prompts]` knobs - retrieval guidance and catalog intent hints.

use std::collections::HashMap;

use crate::ConfigurationLoader::Configuration::Struct::Struct as Configuration;

// ── Prompt knobs (pre-Rust port): retrieve_guidance / ccr_marker_hint /
// catalog_intent_hints previously parsed with no consumer. ──
#[test]
fn TestPromptKnobsResolveFromTomlAndDefault() {
	let _G = crate::Marker::MarkerTemplatesTestGuard();
	let cfg = Configuration {
		raw:"[prompts]\nretrieve_guidance = \"verbose\"\ncatalog_intent_hints = true\nccr_marker_hint = \
		     true\n\n[templates.marker]\nformat = \"{preview}\"\nhint = \"H\"\n"
			.parse()
			.unwrap(),
		Overrides:HashMap::new(),
		ParseFailure:None,
	};
	let mut State = crate::State::AphroditeState::default();
	cfg.ApplyCompression(&mut State);
	assert_eq!(State.RetrieveGuidance, "verbose");
	assert!(State.CatalogIntentHints);
	// Marker registration reached the process-global the builder reads.
	assert!(crate::Marker::MarkerHintEnabled());
	assert_eq!(crate::Marker::RegisteredMarkerFormat().as_deref(), Some("{Preview}"));

	// Absent everywhere => defaults (minimal / off / off / fallback grammar).
	let mut State2 = crate::State::AphroditeState::default();
	Configuration::default().ApplyCompression(&mut State2);
	assert_eq!(State2.RetrieveGuidance, "minimal");
	assert!(!State2.CatalogIntentHints);
	assert!(!crate::Marker::MarkerHintEnabled());
	assert!(crate::Marker::RegisteredMarkerFormat().is_none());
}

#[test]
fn TestPromptKnobsResolveFromEnvironmentOverride() {
	let _G = crate::Marker::MarkerTemplatesTestGuard();
	let mut cfg = Configuration::default();
	cfg.SetOverride("APHRODITE_RETRIEVE_GUIDANCE", "standard");
	cfg.SetOverride("APHRODITE_CATALOG_INTENT_HINTS", "1");
	let mut State = crate::State::AphroditeState::default();
	cfg.ApplyCompression(&mut State);
	assert_eq!(State.RetrieveGuidance, "standard");
	assert!(State.CatalogIntentHints);
}
