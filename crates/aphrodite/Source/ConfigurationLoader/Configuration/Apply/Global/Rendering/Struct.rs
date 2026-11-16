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
	State::Template::Struct::Struct as Template,
};
#[cfg(feature = "proxy")]
use crate::Configuration::Proxy::Multi::Struct::Struct as Multi;
use super::Marker;

/// Everything the process-global renderers take from one configuration document.
///
/// Every field is the value to publish, not a resolution to perform: `None` means
/// "the renderer's own default" (`None` cap = unlimited, `None` templates = the
/// hardcoded builders, `None` family = the renderer's `code_first`), which is a
/// different fact from "absent from the document" and is the only one a publisher
/// can act on.
pub struct Struct {
	/// `[previews] preview_max_chars`; `None` = unlimited.
	pub Limit:Option<u32>,
	/// `[templates.marker]` plus `[prompts] ccr_marker_hint` - the marker grammar
	/// and its hint line, published as one registration.
	pub Marker:Marker::Struct::Struct,
	/// `[templates]`; `None` = the hardcoded per-arm builders stay in force.
	pub Templates:Option<Template>,
	/// `[previews] model_family`; `None` = the renderer's default family.
	pub Family:Option<String>,
}

#[cfg(feature = "proxy")]
impl From<&Multi> for Struct {
	/// From the engine binary's typed document.
	///
	/// `0` is normalised to `None` here rather than in the publisher: the document
	/// spells "unlimited" as `0`, the publisher spells it as `None`, and this is the
	/// boundary between the two spellings.
	fn from(Configuration:&Multi) -> Self {
		Self {
			Limit:Configuration
				.Previews
				.as_ref()
				.and_then(|Previews| Previews.Limit)
				.filter(|Value| *Value != 0),
			Marker:Marker::Struct::Struct {
				Format:Configuration
					.Templates
					.as_ref()
					.and_then(|Templates| Templates.Marker.get("format"))
					.cloned(),
				Hint:Configuration
					.Templates
					.as_ref()
					.and_then(|Templates| Templates.Marker.get("hint"))
					.cloned(),
				Enable:Configuration.Prompts.as_ref().and_then(|Prompts| Prompts.Hint).unwrap_or(false),
			},
			Templates:Configuration.Templates.clone(),
			Family:Configuration.Previews.as_ref().and_then(|Previews| Previews.Family.clone()),
		}
	}
}

impl From<&Configuration> for Struct {
	/// From the Hermes-side document, through the per-knob resolution chain.
	///
	/// The cap and the marker-hint switch have environment spellings and go through
	/// `Resolve::Number`/`Resolve::Bool` so an override outranks the file, exactly as
	/// every other knob on that path does. The family does not: there is no env-var
	/// spelling of a preview family, and inventing one would make the same value
	/// configurable in two places with different precedence.
	fn from(Configuration:&Configuration) -> Self {
		let Templates = Configuration.Template();

		Self {
			Limit:match Configuration.Number("APHRODITE_PREVIEW_MAX_CHARS", "previews", "preview_max_chars", 0) {
				0 => None,
				Value => Some(Value.min(u32::MAX as u64) as u32),
			},
			Marker:Marker::Struct::Struct {
				Format:Templates.as_ref().and_then(|Templates| Templates.Marker.get("format")).cloned(),
				Hint:Templates.as_ref().and_then(|Templates| Templates.Marker.get("hint")).cloned(),
				Enable:Configuration.Bool("APHRODITE_CCR_MARKER_HINT", "prompts", "ccr_marker_hint", false),
			},
			Templates,
			Family:Configuration
				.Section("previews")
				.and_then(|Table| Table.get("model_family"))
				.and_then(|Value| Value.as_str())
				.map(str::to_string),
		}
	}
}
