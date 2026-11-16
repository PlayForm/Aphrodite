//! `[previews] preview_max_chars` reaching the process-global builder.

use std::collections::HashMap;

use super::super::Guard::GlobalsGuards;
use crate::ConfigurationLoader::Configuration::Struct::Struct as Configuration;

// ── Issue #11 WS4: `[previews] preview_max_chars` must reach the
// preview builder (the key was declared-but-unread dead config). ──
#[test]
fn TestApplyGlobalsWiresPreviewMaximumChars() {
	// Hermetic: a stray APHRODITE_PREVIEW_MAX_CHARS in the session env
	// (e.g. left by a manual cap probe) would win the precedence chain
	// and break every assert - and the leaked cap would poison every
	// later preview test. Remove it for the duration of this test.
	let EnvironmentBackup = std::env::var("APHRODITE_PREVIEW_MAX_CHARS").ok();
	unsafe { std::env::remove_var("APHRODITE_PREVIEW_MAX_CHARS") };
	// Publish and assert under all three guards: a sibling test's publication
	// between the two would make the assertion read another document's cap.
	let _Guards = GlobalsGuards();
	// TOML wins over the default (unlimited).
	let cfg = Configuration {
		raw:"[previews]\npreview_max_chars = 77\n".parse().unwrap(),
		Overrides:HashMap::new(),
		ParseFailure:None,
	};
	cfg.ApplyGlobals();
	assert_eq!(crate::Preview::State::Get::Fn(), 77);

	// Env override wins over TOML.
	let mut Cfg2 = Configuration::default();
	Cfg2.SetOverride("APHRODITE_PREVIEW_MAX_CHARS", "123");
	Cfg2.ApplyGlobals();
	assert_eq!(crate::Preview::State::Get::Fn(), 123);

	// Absent/0 -> unlimited (legacy behavior), and restore the global.
	Configuration::default().ApplyGlobals();
	assert_eq!(crate::Preview::State::Get::Fn(), 0);
	match EnvironmentBackup {
		Some(v) => unsafe { std::env::set_var("APHRODITE_PREVIEW_MAX_CHARS", v) },
		None => {},
	}
}
