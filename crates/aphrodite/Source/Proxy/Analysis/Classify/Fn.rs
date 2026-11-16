//! Classify proxy content into the type names the CCR layer and the previews
//! understand.
//!
//! Deliberately string-based and order-sensitive: the first matching family
//! wins, and the returned names are the same ones `ThresholdFor` scales on and
//! the marker prints, so a new name here without a matching arm there silently
//! falls to the base threshold.
//!
//! The chain is the contract, not an implementation detail. `Envelope` runs
//! first because the preview pipeline deliberately excludes Hermes wrapper
//! envelopes from `JSON`; the pipeline runs next; the first-line rules are the
//! last resort for content with no shape of its own. Each step is a fallback in
//! the strict sense - `or_else` runs it only when the previous step declined.

use crate::Proxy::Analysis::Classify::{
	Envelope::Fn::Fn as ClassifyEnvelope,
	FirstLine::Fn::Fn as Line,
	Refine::Fn::Fn as RefineName,
};

/// Detect the content type for adaptive compression strategy.
pub(crate) fn Fn(Content:&str) -> &'static str {
	ClassifyEnvelope(Content)
		.or_else(|| crate::Preview::Detect::SemanticType::Fn(Content).map(|Name| RefineName(Name, Content)))
		.unwrap_or_else(|| Line(Content))
}
