//! `[compression]` appliers - engine thresholds, runtime toggles, debug level
//! and the chain-split bounds.

use std::collections::HashMap;

use super::super::Guard::{CwdGuard, GuardedApplyCompression};
use crate::ConfigurationLoader::Configuration::Struct::Struct as Configuration;

// ── T16 (F3): `ApplyCompression` must read the TOML key/env var that
// actually ships (`ToolThresholdToken`/`APHRODITE_TOOL_THRESHOLD_TOKEN`)
// - the old `ToolThreshold`/`APHRODITE_TOOL_THRESHOLD` names exist in no
// shipped TOML or doc, so wiring this up without the rename would have
// silently resolved to the default forever. ──
#[test]
fn TestApplyCompressionReadsToolThresholdTokenKey() {
	let mut cfg = Configuration::default();
	cfg.SetOverride("APHRODITE_TOOL_THRESHOLD_TOKEN", "777");
	let mut State = crate::State::AphroditeState::default();
	GuardedApplyCompression(&cfg, &mut State);
	assert_eq!(State.ToolThreshold, 777);
}

#[test]
fn TestApplyCompressionFromTomlTable() {
	let cfg = Configuration {
		raw:"[compression]\ntool_threshold_token = 321\n".parse().unwrap(),
		Overrides:HashMap::new(),
		ParseFailure:None,
	};
	let mut State = crate::State::AphroditeState::default();
	GuardedApplyCompression(&cfg, &mut State);
	assert_eq!(State.ToolThreshold, 321);
}

// ── 05-T5 (P1): `[flow] budget_chars` (+ env override) resolves into
// `state.flow_budget_chars`; default is 4000. ──
#[test]
fn TestFlowBudgetFromToml() {
	let cfg = Configuration {
		raw:"[flow]\nbudget_chars = 1234\n".parse().unwrap(),
		Overrides:HashMap::new(),
		ParseFailure:None,
	};
	let mut State = crate::State::AphroditeState::default();
	GuardedApplyCompression(&cfg, &mut State);
	assert_eq!(State.FlowBudgetChars, 1234);

	// Env override wins over TOML.
	let mut Cfg2 = Configuration::default();
	Cfg2.SetOverride("APHRODITE_FLOW_BUDGET_CHARS", "555");
	let mut State2 = crate::State::AphroditeState::default();
	GuardedApplyCompression(&Cfg2, &mut State2);
	assert_eq!(State2.FlowBudgetChars, 555);

	// Absent everywhere => default 4000.
	let mut State3 = crate::State::AphroditeState::default();
	GuardedApplyCompression(&Configuration::default(), &mut State3);
	assert_eq!(State3.FlowBudgetChars, 4000);
}

// ── Poll-worker config flag ──────────────────────────────

#[test]
fn TestPollWorkerEnabledDefaultTrue() {
	let cfg = Configuration::default();
	let mut State = crate::State::AphroditeState::default();
	GuardedApplyCompression(&cfg, &mut State);
	assert!(State.PollWorkerEnabled, "poll_worker must default to true");
}

#[test]
fn TestPollWorkerDisabledViaEnvironmentOverride() {
	let mut cfg = Configuration::default();
	cfg.SetOverride("APHRODITE_POLL_WORKER", "false");
	let mut State = crate::State::AphroditeState::default();
	GuardedApplyCompression(&cfg, &mut State);
	assert!(!State.PollWorkerEnabled);
}

#[test]
fn TestPollWorkerDisabledViaToml() {
	let cfg = Configuration {
		raw:"[compression]\npoll_worker = false\n".parse().unwrap(),
		Overrides:HashMap::new(),
		ParseFailure:None,
	};
	let mut State = crate::State::AphroditeState::default();
	GuardedApplyCompression(&cfg, &mut State);
	assert!(!State.PollWorkerEnabled);
}

// ── DEV-ONLY debug logger (crate::debug, temporary instrumentation):
// `APHRODITE_DEBUG` must be captured into `state.debug_level` at config
// load (0 = off default, 1 = on, 2 = verbose, >2 clamps to 2). ──
#[test]
fn TestAphroditeDebugEnvironmentSetsStateLevel() {
	let _G = CwdGuard.get_or_init(|| std::sync::Mutex::new(())).lock().unwrap();
	const Key:&str = "APHRODITE_DEBUG";
	let Backup = std::env::var(Key).ok();
	unsafe { std::env::remove_var(Key) };

	// Absent => 0 (off - byte-identical behavior when unset).
	let mut State = crate::State::AphroditeState::default();
	GuardedApplyCompression(&Configuration::default(), &mut State);
	assert_eq!(State.DebugLevel, 0, "absent APHRODITE_DEBUG must leave debug off");
	assert!(!crate::Debug::Level::Enabled::Fn::Fn(), "process-global level must mirror 0");

	for (Value, Expected) in [("1", 1u8), ("2", 2u8), ("3", 2u8)] {
		unsafe { std::env::set_var(Key, Value) };
		let mut State = crate::State::AphroditeState::default();
		GuardedApplyCompression(&Configuration::default(), &mut State);
		assert_eq!(State.DebugLevel, Expected, "APHRODITE_DEBUG={Value:?}");
	}

	unsafe { std::env::remove_var(Key) };
	if let Some(v) = Backup {
		unsafe { std::env::set_var(Key, v) };
	}
	// Leave the process-global logger off for the rest of the suite.
	crate::Debug::Level::Set::Fn::Fn(0);
}

// ── Opt-in config auto-reload key: `[compression] auto_reload`
// (env `APHRODITE_AUTO_RELOAD`), default OFF. When true the dylib
// watches aphrodite.toml and re-applies config fields on change. ──
#[test]
fn TestAutoReloadResolution() {
	// TOML true.
	let cfg = Configuration {
		raw:"[compression]\nauto_reload = true\n".parse().unwrap(),
		Overrides:HashMap::new(),
		ParseFailure:None,
	};
	let mut State = crate::State::AphroditeState::default();
	GuardedApplyCompression(&cfg, &mut State);
	assert!(State.AutoReload, "auto_reload must resolve from TOML");

	// Default off.
	let mut State2 = crate::State::AphroditeState::default();
	GuardedApplyCompression(&Configuration::default(), &mut State2);
	assert!(!State2.AutoReload, "auto_reload must default to off");

	// ── Env override wins.
	let mut Cfg3 = Configuration::default();
	Cfg3.SetOverride("APHRODITE_AUTO_RELOAD", "1");
	let mut State3 = crate::State::AphroditeState::default();
	GuardedApplyCompression(&Cfg3, &mut State3);
	assert!(State3.AutoReload, "env override must enable auto_reload");
}
