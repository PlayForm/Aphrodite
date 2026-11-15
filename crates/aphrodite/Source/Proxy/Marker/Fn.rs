//! Choose the marker shape for a piece of content: full or centred preview.
//!
//! "Smart" is only about which slice of the content is worth showing - the
//! marker grammar itself is fixed. The centre excerpt exists because the useful
//! part of a long output is rarely its first line.
//!
//! The preview and the metadata are rendered from ONE analysis value, so the
//! marker cannot show a preview arm for one content family and a metadata block
//! for another.

use crate::Proxy::{
	Analysis::Struct::Struct as ContentAnalysis,
	Format::Fn::Fn as ProxyFormatCCROutput,
	Metadata::Fn::Fn as GenerateMetadata,
};

/// Render the marker the model reads.
pub(crate) fn Fn(hash:&str, Analysis:&ContentAnalysis<'_>, Center:Option<&str>) -> String {
	let Metadata = GenerateMetadata(Analysis);
	// 1.5.0 (REFACTOR-PLAN §7): ALL types route through the single core builder
	// (`crate::Preview::Builder::Entry`), the same one the Hermes hook/FFI path
	// uses, so proxy and hook previews can never drift.
	let Preview = crate::Preview::Builder::Entry::Fn(Analysis.Ct, Analysis.Content);

	ProxyFormatCCROutput(&Preview, Analysis.Ct, &Metadata, Center, hash, Analysis.Content.len())
}
