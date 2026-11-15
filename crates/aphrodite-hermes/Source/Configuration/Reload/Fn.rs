//! Apply a freshly loaded config to one live session, preserving runtime state.
//!
//! The distinction matters: `apply_compression` re-probes the directives
//! directory and reseeds `active_directives`, which the LIVE session owns
//! (runtime swap/add/remove via `aphrodite_directive`). Losing that on a
//! reload would silently reset the model's directive selection mid-session,
//! so the runtime-owned fields are snapshot/restored around the re-apply.

use aphrodite::State::Struct::AphroditeState;

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
	let saved_directives = std::mem::take(&mut state.Directives);
	let saved_active = std::mem::take(&mut state.ActiveDirectives);
	cfg.ApplyCompression(state);
	state.Directives = saved_directives;
	state.ActiveDirectives = saved_active;
	cfg.ApplyPreviews();
	cfg.ApplyTemplates(state);
}
