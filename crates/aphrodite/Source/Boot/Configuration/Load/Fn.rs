//! Parsing the config file (or the CLI) and registering what it configures.

use aphrodite::Configuration::{
	CLI::Struct::Struct as CLI,
	Environment::Bool::Fn::EnvironmentBool,
	Proxy::Multi::Struct::Struct as MultiConfiguration,
};
use clap::Parser;

use super::{
	super::{Path, Source},
	Loaded,
};

/// Load `aphrodite.toml` when one was found, otherwise parse the CLI.
///
/// The registrations in the file arm are process-global on purpose: the preview
/// builder, the marker builder and the template renderer have no
/// `AphroditeState` to read from, so the values are pushed into them once at
/// startup. A key declared in the config structs but never registered here is a
/// no-op knob - which is exactly what `previews.preview_max_chars` was before it
/// was wired.
///
/// The two arms are the two values `Path::Found::Enum` can hold, so "a file was
/// found" and "the CLI is the configuration" cannot both be true and neither can
/// be forgotten.
pub(crate) fn Fn(Resolved:&Path::Resolved::Struct::Struct) -> anyhow::Result<Loaded::Struct::Struct> {
	match Resolved.Found {
		Path::Found::Enum::Enum::Present => {
			let Configuration = MultiConfiguration::Load(&Resolved.Path)?;

			// ── Issue #11 WS4: wire `[previews] preview_max_chars` into the
			// process-global preview builder - the key was declared in the
			// config structs but never read anywhere, so every preview knob
			// was a no-op. Absent -> unlimited (legacy behavior).
			aphrodite::Preview::State::Set::Fn(Configuration.Previews.as_ref().and_then(|p| p.PreviewMaximumChars));

			// ── TOML-driven marker block templates (pre-Rust port): register the
			// `[templates.marker]` format/hint strings and the `[prompts]
			// ccr_marker_hint` knob process-globally, mirroring the preview-cap
			// registration above (the marker builder has no AphroditeState).
			aphrodite::Marker::Format::Const::SetMarkerTemplates(
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
			aphrodite::Template::Family::Set::Fn::Fn(
				Configuration.Previews.as_ref().and_then(|p| p.ModelFamily.clone()),
			);
			aphrodite::Template::Set::Fn::Fn(Configuration.Templates.clone());

			Ok(Loaded::Struct::Struct {
				Source:Source::Enum::Enum::Multi(Configuration),
				LogCompact:EnvironmentBool("APHRODITE_LOG_COMPACT"),
			})
		},
		Path::Found::Enum::Enum::Absent => {
			let CLI = CLI::parse();
			// Resolved before the source takes ownership of `CLI`: the flag has two
			// spellings on this path, `--log-compact` on the command line and the
			// environment variable the file path also honours, and either one turns it on.
			let LogCompact = CLI.LogCompact || EnvironmentBool("APHRODITE_LOG_COMPACT");

			Ok(Loaded::Struct::Struct { Source:Source::Enum::Enum::Standalone(CLI), LogCompact })
		},
	}
}
