//! A reload must update config fields while preserving the runtime-owned
//! directive selection.

use aphrodite::State::Struct::AphroditeState;

use crate::Configuration::Reload;

// ── Config auto-reload: a fresh `Config` re-applied to the session
// state must update config fields (thresholds) while PRESERVING the
// runtime-owned directive selection (map + active list) - the exact
// contract of the opt-in watcher (`Watcher::Apply::Fn::Fn`). CCR stores
// and telemetry are untouched by `apply_compression` itself. ──
#[test]
fn test_config_reload_preserves_directives_and_applies_thresholds() {
	let _g = crate::Session::Guard::Fn::test_guard();

	let Stamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_nanos();
	let Path = std::env::temp_dir().join(format!("aphrodite-cfg-reload-{Stamp}.toml"));
	std::fs::write(&Path, "[compression]\nterminal_threshold = 2048\n").unwrap();
	let Configuration = aphrodite::ConfigurationLoader::Configuration::Struct::Struct::Read(Path.to_str().unwrap());
	let _ = std::fs::remove_file(&Path);
	assert!(Configuration.ParseFailure.is_none());

	let mut State = AphroditeState::default();
	State.Configuration.TerminalThreshold = 1024;
	// Simulate a live session that swapped in a runtime directive set.
	State.Directive.Loaded.insert(
		"focus".into(),
		aphrodite::Directive::Model::Struct { name:"focus".into(), Content:"stay targeted".into() },
	);
	State.Directive.Active = vec!["focus".into()];

	Reload::Fn::Fn(&Configuration, &mut State);

	assert_eq!(State.Configuration.TerminalThreshold, 2048, "reload must apply new thresholds");
	assert!(
		State.Directive.Loaded.contains_key("focus"),
		"reload must preserve the runtime directive set"
	);
	assert_eq!(
		State.Directive.Active,
		vec!["focus".to_string()],
		"reload must preserve active directives"
	);
}
