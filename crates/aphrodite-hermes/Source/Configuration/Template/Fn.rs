//! Build the process-wide config-seeded state template.
//!
//! This is the ONLY place this crate loads `aphrodite.toml`. Every session
//! state is a clone of the resulting template, so the load happens once per
//! process and a subagent spawn inherits the parent's thresholds, directives
//! and templates without touching the filesystem again.

use std::sync::OnceLock;

use aphrodite::State::Struct::AphroditeState;

use crate::Session::Watcher::Fn::spawn_config_watcher;
/// Config-seeded template state, built once per process.
///
/// Initializes from `aphrodite.toml` (report 07 F3/T16) via
/// `ConfigurationLoader::Config` - previously this crate always ran on
/// hardcoded `AphroditeState::default()` values; `Config::load()`/
/// `apply_compression` were the only implementation of the advertised
/// env>TOML>default resolution for this path but had zero call sites anywhere
/// in the repo. Every new session state is a clone of this template, so a
/// subagent spawn inherits the same thresholds/directives/templates as the
/// parent (the single-session behavior is unchanged: the first session IS a
/// clone of the template, so one session behaves exactly as before).
///
/// Test builds skip the TOML load and use plain defaults: `Config::load()`
/// searches `./aphrodite.toml` first, and this crate's own tests run with the
/// workspace root (which HAS a real `aphrodite.toml`) as their working
/// directory - without this split, unit tests would non-hermetically pick up
/// whatever the repo's live config happens to contain instead of the
/// documented defaults they assert against.
pub(crate) fn config_template() -> &'static AphroditeState {
	static TEMPLATE:OnceLock<AphroditeState> = OnceLock::new();
	TEMPLATE.get_or_init(|| {
		#[cfg(not(test))]
		let state = {
			let mut s = AphroditeState::default();
			let cfg = aphrodite::ConfigurationLoader::Configuration::Struct::Struct::Load();
			cfg.ApplyCompression(&mut s);
			// Issue #11 WS4: also push `[previews] preview_max_chars` into
			// the process-global preview builder so the dylib path caps
			// previews exactly like the proxy path (the key was
			// declared-but-unread dead config). Precedence is
			// $APHRODITE_PREVIEW_MAX_CHARS > TOML > default 120; an absent
			// key means unlimited.
			cfg.ApplyPreviews();
			// Pre-Rust port: register `[templates]` + `[previews]
			// model_family` so the dylib path renders TOML preview
			// templates exactly like the engine/proxy paths.
			cfg.ApplyTemplates(&s);
			// Opt-in config auto-reload (`[compression] auto_reload = true`,
			// default off): watch aphrodite.toml and re-apply config fields
			// on change. Only config fields are mutated - session CCR state,
			// directive selection, and telemetry survive a reload (the
			// dropped dylib-binary hot-reload wiped them; fa85dfa).
			if s.AutoReload {
				spawn_config_watcher();
			}
			s
		};
		#[cfg(test)]
		let state = AphroditeState::default();
		state
	})
}
