//! The apply half of the config watcher: a supplied config reaches every
//! registered session, and a parse failure is reported rather than applied.
//!
//! This used to be unreachable from a test - it sat inside the watcher thread
//! and loaded the config itself, so proving it meant writing the repo's real
//! `aphrodite.toml`. It takes the configuration as an argument now.

use crate::Session::{Registry::Struct::session_state, Watcher::Apply::Fn::Fn as apply};

const PROBE_SESSION:&str = "probe-apply";

fn config_from(Toml:&str) -> aphrodite::ConfigurationLoader::Configuration::Struct::Struct {
	let Stamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_nanos();
	let Path = std::env::temp_dir().join(format!("aphrodite-watcher-apply-{Stamp}.toml"));
	std::fs::write(&Path, Toml).unwrap();
	let Configuration = aphrodite::ConfigurationLoader::Configuration::Struct::Struct::LoadFrom(Path.to_str().unwrap());
	let _ = std::fs::remove_file(&Path);

	Configuration
}

// ── A good config reaches the session: the reload must apply new thresholds
// while leaving the runtime-owned directive selection alone (the contract
// `apply_config_reload` owns, asserted here at the watcher's own entry point). ──
#[test]
fn probe_apply_reaches_the_registered_session() {
	let _g = crate::Session::Guard::Fn::test_guard();

	let State = session_state(PROBE_SESSION);
	State
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner)
		.TerminalThreshold = 1024;

	let Configuration = config_from("[compression]\nterminal_threshold = 2048\n");
	assert!(Configuration.ParseFailure.is_none(), "the probe config must parse");

	apply(&Configuration);

	let Applied = State
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner)
		.TerminalThreshold;
	assert_eq!(Applied, 2048, "the watcher's apply must reach a registered session");
}

// ── A broken file is NOT applied and keeps watching: the previous values
// survive, the failure is recorded on every session so `aphrodite_stats` can
// say "defaults in effect (parse failed)", and the next good write clears it. ──
#[test]
fn probe_apply_reports_a_parse_failure_and_keeps_watching() {
	let _g = crate::Session::Guard::Fn::test_guard();

	let State = session_state(PROBE_SESSION);
	{
		let mut Guard = State.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
		Guard.TerminalThreshold = 1024;
		Guard.ConfigurationError = None;
	}

	let Broken = config_from("[compression\nterminal_threshold = 2048\n");
	assert!(Broken.ParseFailure.is_some(), "the probe config must fail to parse");

	let Watching = apply(&Broken);

	let Guard = State.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
	assert!(
		Watching,
		"a parse failure must keep watching: the next successful write clears it"
	);
	assert_eq!(Guard.TerminalThreshold, 1024, "a broken file must keep the previous values");
	assert!(
		Guard.ConfigurationError.is_some(),
		"a broken file must be reported on the session, not silently dropped"
	);
}
