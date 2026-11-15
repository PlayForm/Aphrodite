//! Choose the marker shape for a piece of content: full or centred preview.
//!
//! "Smart" is only about which slice of the content is worth showing - the
//! marker grammar itself is fixed. The centre excerpt exists because the useful
//! part of a long output is rarely its first line.

use crate::Proxy::{Format::Fn::Fn as ProxyFormatCCROutput, Metadata::Fn::Fn as GenerateMetadata};

/// Create a CCR marker with preview and structure for the LLM.
///
/// Uses [`format_ccr_output`] for the output layout. The LLM reads the
/// preview + structure first, then decides whether to call
/// aphrodite_retrieve for the full content.
///
/// 1.5.0 (REFACTOR-PLAN §7): the parallel `proxy_build_preview` is GONE -
/// ALL types route through the single core builder (`crate::Preview::build_preview`),
/// the same one the Hermes hook/FFI path uses, so proxy and hook previews
/// can never drift. LLM-visible marker text converged on core formats.
pub(crate) fn Fn(hash:&str, Content:&str, Ct:&str, Center:Option<&str>) -> String {
	let Size = Content.len();
	let Metadata = GenerateMetadata(Content, Ct);
	let Preview = crate::Preview::BuildPreview(Ct, Content);
	ProxyFormatCCROutput(&Preview, Ct, &Metadata, Center, hash, Size)
}
