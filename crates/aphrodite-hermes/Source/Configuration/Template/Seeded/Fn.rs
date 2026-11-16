//! Build the process-wide state from `aphrodite.toml`, once per process.
//!
//! This is the ONLY place this crate loads `aphrodite.toml`. Every session state
//! is a clone of the resulting template, so the load happens once per process and
//! a subagent spawn inherits the parent's thresholds, directives and templates
//! without touching the filesystem again.

use aphrodite::State::Struct::AphroditeState;

use crate::{
	Debug::Warn,
	Session::Watcher::{self, Start},
};

/// The config-seeded template state, built once per process.
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
/// The auto-reload watcher is started HERE, not by a caller, because this is the
/// one moment the answer is known and the one moment a failure can still be
/// reported to a human: `auto_reload = true` that produced no watcher would
/// otherwise look exactly like `auto_reload = false` for the life of the process.
pub(crate) fn Fn() -> AphroditeState {
	let mut State = AphroditeState::default();
	let Configuration = aphrodite::ConfigurationLoader::Configuration::Struct::Struct::Load();

	Configuration.Startup(&mut State);
	// One publication for the whole process: the preview cap, the marker grammar
	// and the `[templates]` + `[previews] model_family` pair. Hooks, the dylib and
	// the poll worker have no state to read from, so this is how the document
	// reaches them - and publishing the template pair in one critical section is
	// what stops a reader rendering one document's family against another's
	// templates.
	Configuration.Global();

	// Opt-in config auto-reload (`[compression] auto_reload = true`, default
	// off): watch aphrodite.toml and re-apply config fields on change. Only
	// config fields are mutated - session CCR state, directive selection, and
	// telemetry survive a reload (the dropped dylib-binary hot-reload wiped
	// them; fa85dfa).
	let Started = State.Configuration.AutoReload.then(Watcher::Fn::Fn);

	matches!(Started, Some(Start::Enum::Enum::Failed))
		.then_some(Notice)
		.into_iter()
		.for_each(Warn::Fn::Fn);

	State
}

/// Said when `auto_reload = true` produced no watcher, which would otherwise look
/// exactly like `auto_reload = false` for the life of the process.
const Notice:&str = "config auto-reload was requested (auto_reload = true) but its watcher did not start; \
                     aphrodite.toml changes will not be picked up until this process restarts";
