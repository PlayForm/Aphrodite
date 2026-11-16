//! The process-global rendering configuration, as one value.
//!
//! Three renderers read configuration without an `AphroditeState`: the preview
//! builder (its character cap), the marker builder (`[templates.marker]` format and
//! hint) and the template renderer (`[templates]` plus the `[previews] model_family`
//! selector). Hooks, the dylib entry points and the poll worker all run on paths that
//! have no state to read from, so those values have to be pushed into process globals
//! once per load.
//!
//! Before this value existed the push was written out four times - the engine's
//! startup load, the dylib's init, the dylib's auto-reload and the proxy's
//! `/reload` - each covering a slightly different subset of the three renderers, and
//! two of them setting the template pair through two setters that took the registry
//! write lock separately. A reader could therefore render a `model_family` from one
//! document against templates from another.
//!
//! As a value there is one description of what the globals should be, built from
//! whichever document type the caller holds, and one publisher that writes it.

use crate::{
	ConfigurationLoader::Configuration::Struct::Struct as Configuration,
	State::Template::Struct::TemplatesConfiguration,
};
#[cfg(feature = "proxy")]
use crate::Configuration::Proxy::Multi::Struct::Struct as MultiConfiguration;

/// Everything the process-global renderers take from one configuration document.
///
/// Every field is the value to publish, not a resolution to perform: `None` means
/// "the renderer's own default" (`None` cap = unlimited, `None` templates = the
/// hardcoded builders, `None` family = the renderer's `code_first`), which is a
/// different fact from "absent from the document" and is the only one a publisher
/// can act on.
pub struct Struct {
	/// `[previews] preview_max_chars`; `None` = unlimited.
	pub PreviewMaximum:Option<u32>,
	/// `[templates.marker] format`; `None` = the canonical fallback grammar.
	pub MarkerFormat:Option<String>,
	/// `[templates.marker] hint`; `None` = no hint line.
	pub MarkerHint:Option<String>,
	/// `[prompts] ccr_marker_hint`.
	pub MarkerHintEnabled:bool,
	/// `[templates]`; `None` = the hardcoded per-arm builders stay in force.
	pub Templates:Option<TemplatesConfiguration>,
	/// `[previews] model_family`; `None` = the renderer's default family.
	pub Family:Option<String>,
}

#[cfg(feature = "proxy")]
impl From<&MultiConfiguration> for Struct {
	/// From the engine binary's typed document.
	///
	/// `0` is normalised to `None` here rather than in the publisher: the document
	/// spells "unlimited" as `0`, the publisher spells it as `None`, and this is the
	/// boundary between the two spellings.
	fn from(Configuration:&MultiConfiguration) -> Self {
		Self {
			PreviewMaximum:Configuration
				.Previews
				.as_ref()
				.and_then(|p| p.PreviewMaximumChars)
				.filter(|Maximum| *Maximum != 0),
			MarkerFormat:Configuration.Templates.as_ref().and_then(|t| t.Marker.get("format")).cloned(),
			MarkerHint:Configuration.Templates.as_ref().and_then(|t| t.Marker.get("hint")).cloned(),
			MarkerHintEnabled:Configuration.Prompts.as_ref().and_then(|p| p.CCRMarkerHint).unwrap_or(false),
			Templates:Configuration.Templates.clone(),
			Family:Configuration.Previews.as_ref().and_then(|p| p.ModelFamily.clone()),
		}
	}
}

impl From<&Configuration> for Struct {
	/// From the Hermes-side document, through the per-knob resolution chain.
	///
	/// The cap and the marker-hint switch have environment spellings and go through
	/// `GetU64`/`GetBool` so an override outranks the file, exactly as every other
	/// knob on that path does. The family does not: there is no env-var spelling of a
	/// preview family, and inventing one would make the same value configurable in
	/// two places with different precedence.
	fn from(Configuration:&Configuration) -> Self {
		let Templates = Configuration.GetTemplates();

		Self {
			PreviewMaximum:match Configuration.GetU64("APHRODITE_PREVIEW_MAX_CHARS", "previews", "preview_max_chars", 0)
			{
				0 => None,
				Maximum => Some(Maximum.min(u32::MAX as u64) as u32),
			},
			MarkerFormat:Templates.as_ref().and_then(|t| t.Marker.get("format")).cloned(),
			MarkerHint:Templates.as_ref().and_then(|t| t.Marker.get("hint")).cloned(),
			MarkerHintEnabled:Configuration.GetBool("APHRODITE_CCR_MARKER_HINT", "prompts", "ccr_marker_hint", false),
			Templates,
			Family:Configuration
				.Section("previews")
				.and_then(|s| s.get("model_family"))
				.and_then(|v| v.as_str())
				.map(str::to_string),
		}
	}
}
