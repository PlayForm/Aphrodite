//! Compose a search envelope's payload into the value the classifier reads.

use serde_json::{Map, Value};

use crate::Tool::Unwrap::Search::{Payload, Total};

/// Rebuild a search envelope's matches as grep-style `path:line:content` rows.
///
/// Returns `Some((body, ccr_type))` where `ccr_type` is `"search"` for real rows
/// and `"text"` for the count-only fallback (`"N total"`), or `None` when the
/// object is not a search envelope at all.
///
/// This leaf renders; it decides nothing. Which form arrived, and whether the
/// object is a search at all, is `Payload::Resolve::Fn::Fn`'s decision, and the
/// match below is total over that value - so a new payload form cannot reach the
/// classifier without a rendering arm here.
///
/// The cheap `total_count` guard lives in the resolver, ahead of any row work:
/// an envelope that would have been rejected never pays for normalisation.
pub(crate) fn Fn(Object:&Map<String, Value>) -> Option<(String, &'static str)> {
	match Payload::Resolve::Fn::Fn(Object)? {
		Payload::Enum::Enum::Rows(Rows) => Some((Rows.join("\n"), "search")),
		Payload::Enum::Enum::Total { Count, Truncated } => Some((Total::Fn::Fn(Count, Truncated), "text")),
	}
}
