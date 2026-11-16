//! The `[templates]` configuration section.
//!
//! Pre-Rust port: each preview family maps content types to `{variable}`
//! format strings; resolution order is
//! family[ctype] -> reverse[ctype] -> family[_default] -> hardcoded builder.
//!
//! Defined in the state module (not in the feature-gated configuration
//! module) so the dylib path - which is built without the `Proxy` feature -
//! can carry it. Every field is `#[serde(default)]`: an absent section must
//! deserialize to an empty table rather than failing the whole config load.

use std::collections::HashMap;

/// The `[templates]` section of `aphrodite.toml`: preview families, marker
/// block, prompt templates, reverse key map.
///
/// The field names are the TOML keys and are pinned by `#[serde(rename = ...)]`,
/// so renaming a field here cannot rename a key. An absent section, or an absent
/// key inside it, deserializes to an empty table - which is the signal that the
/// hardcoded per-arm builders stay in force.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct Struct {
	#[serde(default)]
	#[serde(rename = "preview")]
	pub Preview:HashMap<String, HashMap<String, String>>,
	#[serde(default)]
	#[serde(rename = "marker")]
	pub Marker:HashMap<String, String>,
	#[serde(default)]
	#[serde(rename = "prompts")]
	pub Prompts:HashMap<String, String>,
	#[serde(default)]
	#[serde(rename = "reverse")]
	pub Reverse:HashMap<String, String>,
}
