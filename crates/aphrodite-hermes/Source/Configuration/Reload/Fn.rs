//! Apply a freshly loaded config to one live session.

use aphrodite::State::Struct::AphroditeState;

/// Re-apply a freshly loaded config to a live session.
///
/// `ApplyReload` is the narrowed pass: it runs every section a reload has authority
/// over and does not touch the directive set, which the live session owns (runtime
/// swap/add/remove via `aphrodite_directive`). The old shape here was
/// snapshot-and-restore around the full startup pass - it preserved those fields, but
/// only by letting the reload overwrite them first and putting them back afterwards,
/// which meant this path held the authority to destroy them.
///
/// The process-global rendering settings are NOT published here. They are one
/// document's opinion about the whole process, so they belong to the caller that
/// performs one reload - publishing them per session made the number of publications
/// depend on how many session locks happened to be free.
pub(crate) fn apply_config_reload(
	cfg:&aphrodite::ConfigurationLoader::Configuration::Struct::Struct,
	state:&mut AphroditeState,
) {
	cfg.ApplyReload(state);
}
