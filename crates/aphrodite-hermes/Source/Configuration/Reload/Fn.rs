//! Apply a freshly loaded config to one live session, preserving runtime state.
//!
//! The distinction matters: `apply_compression` re-probes the directives
//! directory and reseeds `active_directives`, which the LIVE session owns
//! (runtime swap/add/remove via `aphrodite_directive`). Losing that on a
//! reload would silently reset the model's directive selection mid-session,
//! so the runtime-owned fields are snapshot/restored around the re-apply.

use aphrodite::State::AphroditeState;

/// Re-apply a freshly loaded config to the session state, preserving the
/// runtime-owned fields. `apply_compression` re-probes the directives
/// directory and reseeds `active_directives` - the live session owns those
/// (runtime swap/add/remove via `aphrodite_directive`), so they are
/// snapshot/restored around the re-apply. CCR stores, telemetry, and every
/// other runtime field are untouched by `apply_compression` itself.
pub(crate) fn apply_config_reload(
	cfg:&aphrodite::ConfigurationLoader::Configuration::Struct::Struct,
	state:&mut AphroditeState,
) {
	let saved_directives = std::mem::take(&mut state.directives);
	let saved_active = std::mem::take(&mut state.active_directives);
	cfg.apply_compression(state);
	state.directives = saved_directives;
	state.active_directives = saved_active;
	cfg.apply_previews();
	cfg.apply_templates(state);
}
