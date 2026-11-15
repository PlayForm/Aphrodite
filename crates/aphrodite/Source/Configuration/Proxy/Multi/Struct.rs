//! The top-level `aphrodite.toml` document.

use crate::Configuration::Proxy::{
	Compression::Struct::Struct as CompressionConfiguration,
	Defaults::Struct::Struct as Defaults,
	Previews::Struct::Struct as PreviewsConfiguration,
	Prompts::Struct::Struct as PromptsConfiguration,
	Proxy::Struct::Struct as ProxyConfiguration,
};

/// Multi-proxy configuration loaded from aphrodite.toml.
///
/// `Proxies` defaults to an empty list when the `[proxies]` table is
/// absent, so a hook-only configuration (no proxy definitions) parses
/// cleanly instead of failing with `missing field 'proxies'`.
///
/// Every other section is optional and independent: a file may carry only
/// `[compression]` (a hook-only install), only `[[proxies]]` (a proxy-only
/// install), or both. Deserialisation is strict about types but tolerant of
/// absence - a file that does not parse is the loader's problem, not this
/// type's.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Struct {
	/// Shared fallbacks for every `[[proxies]]` entry.
	#[serde(rename = "defaults")]
	pub Defaults:Option<Defaults>,
	/// Proxy definitions; absent table = no proxies, not an error.
	#[serde(default)]
	#[serde(rename = "proxies")]
	pub Proxies:Vec<ProxyConfiguration>,
	/// `[compression]` knobs (thresholds, caps, engine window).
	#[serde(rename = "compression")]
	pub Compression:Option<CompressionConfiguration>,
	/// `[previews]` knobs (templates, structure maps, preview cap).
	#[serde(rename = "previews")]
	pub Previews:Option<PreviewsConfiguration>,
	/// `[prompts]` knobs (retrieval guidance, marker hints).
	#[serde(rename = "prompts")]
	pub Prompts:Option<PromptsConfiguration>,
	/// `[templates]` - preview format strings and prompt text.
	#[serde(rename = "templates")]
	pub Templates:Option<crate::State::TemplatesConfiguration>,
}
