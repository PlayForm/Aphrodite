//! Which regime a length falls into.
//!
//! Ordered, and the order is the policy: the compression threshold is the
//! higher bar, so it is tested first and a payload above it never reaches the
//! inline branch. `or_else` keeps the second test from running once the first
//! has claimed the payload.

use crate::Proxy::Compress::Chat::Sizing::Enum::Enum as ContentSizing;

/// Classify a content length against the two thresholds.
pub(crate) fn Fn(Length:usize, Threshold:usize, InlineThreshold:usize) -> ContentSizing {
	(Length > Threshold)
		.then_some(ContentSizing::Compress)
		.or_else(|| (Length > InlineThreshold).then_some(ContentSizing::Inline))
		.unwrap_or(ContentSizing::Skip)
}
