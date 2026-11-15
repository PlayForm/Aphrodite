//! Parsing the config file (or the CLI) and registering what it configures.
//!
//! The registrations are process-global on purpose: the preview builder, the
//! marker builder and the template renderer have no `AphroditeState` to read
//! from, so the values are pushed into them once at startup. A key that is
//! declared in the config structs but never registered here is a no-op knob -
//! which is exactly what `previews.preview_max_chars` was before it was wired.

use aphrodite::Configuration::{CLI::Struct::Struct as CLI, Proxy::Multi::Struct::Struct as MultiConfiguration};
use clap::Parser;

/// Load `aphrodite.toml` when present, otherwise parse the CLI.
///
/// Returns `(multi_config, cli_fallback, log_compact)` - exactly one of the
/// first two is `Some`. `LogCompact` is resolved here rather than from
/// `MultiConfig::resolve()` so the tracing subscriber can be installed before
/// that runs and none of its diagnostics are dropped.
pub fn Load(
	ConfigurationPath:&str,
	UseMultiConfiguration:bool,
) -> anyhow::Result<(Option<MultiConfiguration>, Option<CLI>, bool)> {
	if UseMultiConfiguration {
		let Configuration = MultiConfiguration::Load(ConfigurationPath)?;
		// ── Issue #11 WS4: wire `[previews] preview_max_chars` into the
		// process-global preview builder - the key was declared in the
		// config structs but never read anywhere, so every preview knob
		// was a no-op. Absent -> unlimited (legacy behavior).
		aphrodite::Preview::SetPreviewMaximumChars(Configuration.Previews.as_ref().and_then(|p| p.PreviewMaximumChars));
		// ── TOML-driven marker block templates (pre-Rust port): register the
		// `[templates.marker]` format/hint strings and the `[prompts]
		// ccr_marker_hint` knob process-globally, mirroring the preview-cap
		// registration above (the marker builder has no AphroditeState).
		aphrodite::Marker::SetMarkerTemplates(
			Configuration
				.Templates
				.as_ref()
				.and_then(|t| t.Marker.get("format"))
				.map(String::as_str),
			Configuration
				.Templates
				.as_ref()
				.and_then(|t| t.Marker.get("hint"))
				.map(String::as_str),
			Configuration.Prompts.as_ref().and_then(|p| p.CCRMarkerHint).unwrap_or(false),
		);
		// ── Pre-Rust port: register `[templates]` + `[previews]
		// model_family` into the process-global template renderer so
		// `[templates.preview.<family>.<ctype>]` format strings render
		// instead of the hardcoded per-arm builders. Absent -> hardcoded.
		aphrodite::Template::Family::Set::Fn::Fn(Configuration.Previews.as_ref().and_then(|p| p.ModelFamily.clone()));
		aphrodite::Template::Set::Fn::Fn(Configuration.Templates.clone());
		let LogCompact = aphrodite::Configuration::EnvironmentBool("APHRODITE_LOG_COMPACT");
		Ok((Some(Configuration), None, LogCompact))
	} else {
		let CLI = CLI::parse();
		let LogCompact = CLI.LogCompact || aphrodite::Configuration::EnvironmentBool("APHRODITE_LOG_COMPACT");
		Ok((None, Some(CLI), LogCompact))
	}
}
