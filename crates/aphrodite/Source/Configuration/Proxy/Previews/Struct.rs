//! `[previews]` - model-aware templates and code structure maps.

/// The `[previews]` table.
///
/// Preview behaviour is presentation only: a wrong value here changes what the
/// model sees of a stored payload, never what is stored. `PreviewMaximumChars`
/// is the exception that proves it - it bounds the rendered preview string,
/// and 0/absent means unlimited rather than "render nothing".
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Struct {
	/// Preview family to render with (`code_first`, ...); absent = default.
	pub ModelFamily:Option<String>,
	/// Render a code-structure map instead of raw lines for source files.
	pub CodeStructureMap:Option<bool>,
	/// Cap on the rendered preview in characters; 0 = unlimited.
	pub PreviewMaximumChars:Option<u32>,
	/// Line count for Rust previews.
	pub RustPreviewLines:Option<u32>,
}
