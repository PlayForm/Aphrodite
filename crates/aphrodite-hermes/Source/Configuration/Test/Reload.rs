//! A reload must update config fields while preserving the runtime-owned
//! directive selection.

use aphrodite::State::Struct::AphroditeState;

use crate::Configuration::Reload::Fn::apply_config_reload;

// ── Config auto-reload: a fresh `Config` re-applied to the session
// state must update config fields (thresholds) while PRESERVING the
// runtime-owned directive selection (map + active list) - the exact
// contract of the opt-in watcher (spawn_config_watcher). CCR stores
// and telemetry are untouched by `apply_compression` itself. ──
#[test]
fn test_config_reload_preserves_directives_and_applies_thresholds() {
	let _g = crate::Session::Guard::Fn::test_guard();

	let stamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_nanos();
	let path = std::env::temp_dir().join(format!("aphrodite-cfg-reload-{stamp}.toml"));
	std::fs::write(&path, "[compression]\nterminal_threshold = 2048\n").unwrap();
	let cfg = aphrodite::ConfigurationLoader::Configuration::Struct::Struct::LoadFrom(path.to_str().unwrap());
	let _ = std::fs::remove_file(&path);
	assert!(cfg.ParseFailure.is_none());

	let mut state = AphroditeState::default();
	state.TerminalThreshold = 1024;
	// Simulate a live session that swapped in a runtime directive set.
	state.Directives.insert(
		"focus".into(),
		aphrodite::Directive::Model::Struct { name:"focus".into(), Content:"stay targeted".into() },
	);
	state.ActiveDirectives = vec!["focus".into()];

	apply_config_reload(&cfg, &mut state);

	assert_eq!(state.TerminalThreshold, 2048, "reload must apply new thresholds");
	assert!(
		state.Directives.contains_key("focus"),
		"reload must preserve the runtime directive set"
	);
	assert_eq!(
		state.ActiveDirectives,
		vec!["focus".to_string()],
		"reload must preserve active directives"
	);
}
