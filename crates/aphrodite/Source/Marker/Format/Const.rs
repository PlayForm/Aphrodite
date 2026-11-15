//! The process-global marker template registration.
//!
//! TOML-driven marker block templates (pre-Rust port of
//! `plugins/aphrodite/_core/template.py` `_render_marker_tmpl`):
//! `[templates.marker] format` + `Hint` strings, with the `{hint}` var only
//! populated when the `[prompts] ccr_marker_hint` knob is enabled.
//!
//! The registration is process-global because the marker builder runs on paths
//! that have no `AphroditeState` (hooks, dylib, poll worker). That is the same
//! reason `Preview::set_preview_max_chars` is global - and the same hazard: a
//! hot-reload that removes `[templates]` must reset the format to `None`, or a
//! stale template stays in force for the life of the process.

use std::sync::{
	RwLock,
	atomic::{AtomicBool, Ordering},
};

/// Registered `[templates.marker] format` string; `None` = the canonical
/// fallback grammar.
static MarkerFormat:RwLock<Option<String>> = RwLock::new(None);
/// Registered `[templates.marker] hint` text; emitted only when the
/// `CCRMarkerHint` knob is enabled (default off).
static MarkerHint:RwLock<String> = RwLock::new(String::new());
/// `[prompts] ccr_marker_hint` knob - hint text is emitted only when true.
static MarkerHintFlag:AtomicBool = AtomicBool::new(false);

/// Register the TOML-driven marker block templates.
///
/// Mirrors the `Preview::set_preview_max_chars` registration pattern. Set
/// by: the engine binary right after `MultiConfig::load` (Main.rs), the
/// proxy `/reload` handler (Proxy/), and
/// `ConfigurationLoader::Configuration::Struct::Struct::apply_compression` at dylib init. `None` format
/// (or an absent `[templates]` section) resets to the fixed
/// `<<<CCR:{hash}|{type}|{size}>>>` fallback so a hot-reload that removes
/// the templates never leaves a stale format in force.
pub fn SetMarkerTemplates(Format:Option<&str>, Hint:Option<&str>, HintEnabled:bool) {
	*MarkerFormat.write().unwrap_or_else(std::sync::PoisonError::into_inner) = Format.map(|s| s.to_string());
	*MarkerHint.write().unwrap_or_else(std::sync::PoisonError::into_inner) = Hint.unwrap_or("").to_string();
	MarkerHintFlag.store(HintEnabled, Ordering::Relaxed);
}

/// Currently registered marker format (test/visibility helper).
pub(crate) fn RegisteredMarkerFormat() -> Option<String> {
	MarkerFormat.read().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
}

/// Whether the `CCRMarkerHint` knob is enabled (test/visibility helper).
pub(crate) fn MarkerHintEnabled() -> bool { MarkerHintFlag.load(Ordering::Relaxed) }

/// The hint text to emit right now: empty unless the `CCRMarkerHint` knob
/// is enabled AND a hint string is registered (port of Python's
/// `hint = hint_str if CCR_MARKER_HINT and hint_str else ""`).
pub(super) fn CurrentHint() -> String {
	if !MarkerHintEnabled() {
		return String::new();
	}
	MarkerHint.read().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
}

/// Serializes tests across modules that mutate the process-global marker
/// templates (`cargo test` runs module test-bodies concurrently; the
/// registration is process-wide, so config_loader's `ApplyCompression`
/// tests and this module's template tests must not interleave).
#[cfg(test)]
pub(crate) fn MarkerTemplatesTestGuard() -> std::sync::MutexGuard<'static, ()> {
	static G:std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
	G.get_or_init(|| std::sync::Mutex::new(()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner)
}
