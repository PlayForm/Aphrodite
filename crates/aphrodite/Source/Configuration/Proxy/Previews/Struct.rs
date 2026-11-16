//! `[previews]` - model-aware templates and code structure maps.

/// The `[previews]` table.
///
/// Preview behaviour is presentation only: a wrong value here changes what the
/// model sees of a stored payload, never what is stored. `Limit` is the
/// exception that proves it - it bounds the rendered preview string, and
/// 0/absent means unlimited rather than "render nothing".
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Struct {
	/// `[previews] model_family`; `None` = the renderer's default family
	/// (`code_first`).
	#[serde(rename = "model_family")]
	pub Family:Option<String>,
	/// `[previews] code_structure_map`; render a code-structure map instead of
	/// raw lines for source files. `None` = off.
	#[serde(rename = "code_structure_map")]
	pub Structure:Option<bool>,
	/// `[previews] preview_max_chars`; cap on the rendered preview in
	/// characters. `None` or `0` = unlimited.
	#[serde(rename = "preview_max_chars")]
	pub Limit:Option<u32>,
	/// `[previews] rust_preview_lines`; line count for Rust previews.
	/// `None` = the renderer's default.
	#[serde(rename = "rust_preview_lines")]
	pub Lines:Option<u32>,
}
