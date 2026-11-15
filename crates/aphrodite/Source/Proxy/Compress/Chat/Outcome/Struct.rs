//! The rewritten body, plus the per-message decisions that produced it.
//!
//! The body and the decisions travel together because the responder needs both
//! and they must not disagree: whether to set `X-Aphrodite-Compressed`, whether
//! to count the request as compressed, and how many bytes the rewrites saved are
//! all read from `Rewrites`, which is the same list that produced the body.
//!
//! `Compressed` and `Accounting` are projections of that list, so a caller
//! cannot ask "did anything change" and get an answer from a different source
//! than the bytes it is about to emit.

use crate::Proxy::{
	Compress::Chat::Rewrite::Enum::Enum as ContentRewrite,
	Measurement::Wire::Struct::Struct as WireAccounting,
};

/// The rewritten response and the decisions behind it.
pub(crate) struct Struct {
	/// The response body after every replacement was applied.
	pub(crate) Response:serde_json::Value,
	/// One decision per choice, in choice order.
	pub(crate) Rewrites:Vec<ContentRewrite>,
}

impl Struct {
	/// Did any message change at all - replaced by a marker, or stored inline
	/// with the wire response untouched?
	pub(crate) fn Rewritten(&self) -> bool {
		self.Rewrites
			.iter()
			.any(|Rewrite| !matches!(Rewrite, ContentRewrite::Unchanged(_)))
	}

	/// Did any message get replaced by a marker?
	pub(crate) fn Compressed(&self) -> bool {
		self.Rewrites
			.iter()
			.any(|Rewrite| matches!(Rewrite, ContentRewrite::Replaced { .. }))
	}

	/// What the replacements saved, summed over the messages that were replaced.
	pub(crate) fn Accounting(&self) -> WireAccounting {
		self.Rewrites
			.iter()
			.filter_map(|Rewrite| {
				match Rewrite {
					ContentRewrite::Replaced { Accounting, .. } => Some(Accounting),
					_ => None,
				}
			})
			.fold(WireAccounting { OriginalBytes:0, EmittedBytes:0 }, |Total, Accounting| {
				WireAccounting {
					OriginalBytes:Total.OriginalBytes.saturating_add(Accounting.OriginalBytes),
					EmittedBytes:Total.EmittedBytes.saturating_add(Accounting.EmittedBytes),
				}
			})
	}
}
