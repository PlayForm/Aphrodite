//! `Load` tests - what a file on disk produces, and what a missing or broken
//! file produces instead.

use super::Guard::GuardedApplyCompression;
use crate::ConfigurationLoader::Configuration::Struct::Struct as Configuration;

#[test]
fn TestLoadFromValidAndMissing() {
	let Stamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_nanos();
	let Valid = std::env::temp_dir().join(format!("aphrodite-cfg-valid-{Stamp}.toml"));
	std::fs::write(&Valid, "[compression]\nenabled = false\n").unwrap();
	let cfg = Configuration::LoadFrom(Valid.to_str().unwrap());
	let _ = std::fs::remove_file(&Valid);
	assert!(
		!cfg.GetBool("NONEXISTENT", "compression", "enabled", true),
		"a valid file must parse and win over the default"
	);

	// Missing path: defaults, no panic.
	let cfg = Configuration::LoadFrom(&format!("/nonexistent/aphrodite-{Stamp}.toml"));
	assert!(cfg.GetBool("NONEXISTENT", "compression", "enabled", true));
}

// ── Parse-failure self-diagnosis: a broken file must record itself on
// the Config (and via `ApplyCompression` on the state) so
// `AphroditeStats` can surface "defaults in effect (parse failed)" -
// the tracing warn is a no-op in the Hermes dylib (no subscriber), so
// the recorded field is the only guaranteed-visible diagnostic. ──
#[test]
fn TestParseFailureRecordedOnConfigurationAndState() {
	let Stamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_nanos();
	let Broken = std::env::temp_dir().join(format!("aphrodite-cfg-broken-recorded-{Stamp}.toml"));
	std::fs::write(&Broken, "[compression\nenabled = false\n").unwrap();

	let cfg = Configuration::LoadFrom(Broken.to_str().unwrap());
	let _ = std::fs::remove_file(&Broken);

	let Pf = cfg.ParseFailure.as_ref().expect("parse failure must be recorded");
	assert!(
		Pf.contains(Broken.to_str().unwrap()),
		"recorded failure must name the broken path: {Pf}"
	);
	assert!(
		Pf.contains("TOML parse error"),
		"recorded failure must carry the parse error: {Pf}"
	);

	// `AphroditeStats` self-diagnosis surface.
	let mut State = crate::State::Struct::AphroditeState::default();
	GuardedApplyCompression(&cfg, &mut State);
	let Ce = State
		.ConfigurationError
		.as_deref()
		.expect("config_error must be set on the state");
	assert!(
		Ce.contains(Broken.to_str().unwrap()),
		"config_error must name the broken path: {Ce}"
	);
}

// ── Multibyte content in comments is valid TOML: an em dash inside a
// comment must parse and resolve - the fallback-to-defaults trigger is
// a *broken* file, never non-ASCII bytes (report claim check). ──
#[test]
fn TestLoadFromMultibyteCommentParses() {
	let Stamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_nanos();
	let path = std::env::temp_dir().join(format!("aphrodite-cfg-multibyte-{Stamp}.toml"));
	std::fs::write(
		&path,
		"[compression]\nterminal_threshold = 16384   # bytes; was default 1024 - raised\n",
	)
	.unwrap();
	let cfg = Configuration::LoadFrom(path.to_str().unwrap());
	let _ = std::fs::remove_file(&path);

	assert!(
		cfg.ParseFailure.is_none(),
		"a multibyte comment must not fail the parse: {:?}",
		cfg.ParseFailure
	);
	let mut State = crate::State::Struct::AphroditeState::default();
	GuardedApplyCompression(&cfg, &mut State);
	assert_eq!(State.TerminalThreshold, 16384);
	assert!(State.ConfigurationError.is_none());
}
