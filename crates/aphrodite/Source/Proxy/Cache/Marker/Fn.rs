//! The cache-mode marker text: the same `<<<CCR:...>>>` shape, cache flavour.
//!
//! Cache mode passes most content through and only compresses very large
//! outputs, so its marker is produced by the same formatter as the token mode's;
//! only the threshold and the preview slice differ. Two marker shapes for one
//! protocol would make the model's retrieval vocabulary wrong in one of the two
//! modes.

use crate::Proxy::{Analysis::Struct::Struct as ContentAnalysis, Format::Fn::Fn as ProxyFormatCCROutput};

/// Cache-mode CCR output - a 512-character head plus the marker.
pub(crate) fn Fn(hash:&str, Analysis:&ContentAnalysis<'_>, Center:Option<&str>) -> String {
	let Preview:String = Analysis.Content.chars().take(512).collect();

	ProxyFormatCCROutput(&Preview, Analysis.Ct, "", Center, hash, Analysis.Content.len())
}
