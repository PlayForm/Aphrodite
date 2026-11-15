//! The cache-mode marker text: the same `<<<CCR:...>>>` shape, cache flavour.
//!
//! Cache mode passes most content through and only compresses very large
//! outputs, so its marker is produced by the same formatter as the token mode's;
//! only the threshold differs. Two marker shapes for one protocol would make the
//! model's retrieval vocabulary wrong in one of the two modes.

use crate::Proxy::Format::Fn::Fn as ProxyFormatCCROutput;

/// Cache-mode CCR output - preview + marker, same template.
pub(crate) fn Fn(hash:&str, Content:&str, Ct:&str, Center:Option<&str>) -> String {
	let Size = Content.len();
	let Preview:String = Content.chars().take(512).collect();
	ProxyFormatCCROutput(&Preview, Ct, "", Center, hash, Size)
}
