//! The cache-mode marker text: the same `<<<CCR:...>>>` shape, cache flavour.
//!
//! Cache mode passes most content through and only compresses very large
//! outputs, so its marker is produced by the same formatter as the token mode's;
//! only the threshold and the preview slice differ. Two marker shapes for one
//! protocol would make the model's retrieval vocabulary wrong in one of the two
//! modes.

use crate::Proxy::{
	Analysis::Struct::Struct as ContentAnalysis,
	Body::Chars::Fn::Fn as CharPrefix,
	Format::Fn::Fn as Format,
};

/// Cache-mode CCR output - a 512-CHARACTER head plus the marker.
pub(crate) fn Fn(hash:&str, Analysis:&ContentAnalysis<'_>, Center:Option<&str>) -> String {
	// Borrowed, not collected: the formatter takes a `&str`, and the budget is in
	// characters so a CJK preview is not cut to a third of the length an ASCII one
	// gets.
	let Preview = CharPrefix(Analysis.Content(), 512);

	Format(Preview, Analysis.Ct(), "", Center, hash, Analysis.Size())
}
