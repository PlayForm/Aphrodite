//! Serializers for the process-global state the test areas share.

use crate::{ConfigurationLoader::Configuration::Struct::Struct as Configuration, State::Struct::AphroditeState};

/// Tests that mutate the process cwd / env serialize on this guard -
/// `ApplyCompression`'s directives discovery reads cwd + env vars, so two
/// such tests running in parallel could observe each other's state.
pub(super) static CwdGuard:std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();

/// Serialize `ApplyCompression` against the marker-template tests:
/// `ApplyCompression` registers the process-global marker templates
/// (`[templates.marker]` + `CCRMarkerHint`), which the marker module's
/// tests mutate under `marker_templates_test_guard()` - both sides must
/// hold the same mutex or `cargo test`'s concurrent module bodies race.
pub(super) fn GuardedApplyCompression(cfg:&Configuration, State:&mut AphroditeState) {
	let _G = crate::Marker::Format::Const::MarkerTemplatesTestGuard();
	cfg.ApplyCompression(State);
}
