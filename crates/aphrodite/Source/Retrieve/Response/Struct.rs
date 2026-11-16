//! What the retrieve endpoint answers with.

use serde::Serialize;

use crate::Retrieve::Progress::Struct::Struct as LiveProgress;

/// Response body for `/retrieve`. `Source` reports which backend served the
/// content (e.g. `"inline_ccr"`, `"ccr"`, `"none"`) for observability.
#[derive(Debug, Serialize)]
pub struct Struct {
	/// `true` when `Content` carries a payload (a full document or a window of
	/// one); `false` on every error shape.
	#[serde(rename = "found")]
	pub Found:bool,
	/// The resolved payload, or `None` when the lookup failed.
	#[serde(rename = "content")]
	pub Content:Option<String>,
	/// Which store served it: `"inline_ccr"` when this session's own inline entry
	/// answered, `"ccr"` when the CCR backend did, `"none"` on a lookup failure.
	/// Produced by [`crate::Retrieve::Handle::Served::Enum::Enum::Wire`], so it cannot
	/// disagree with the store that produced the content.
	#[serde(rename = "source")]
	pub Source:String,
	/// Human-readable failure reason; `None` on success.
	pub error:Option<String>,
	/// `true` when `Content` is a partial window of a larger stored document -
	/// because of `Offset`/`Limit`, or because an explicit `Limit` hit the
	/// 10,000-line server cap (02-F5). The exact retrieval (wire `offset: 0`,
	/// `limit: 0`, which `Page::Mode` represents as its own variant rather than a
	/// sentinel) always returns the FULL document: that is the round-trip contract,
	/// since retrieval must return the exact original bytes so the body hashes to
	/// its own marker hash. A `[lines a-b/total]` header is prepended to `Content`
	/// whenever it IS a window; this field lets a caller detect truncation without
	/// parsing it.
	#[serde(rename = "truncated")]
	pub Truncated:bool,
	/// Live-container progress; present only when the resolved entry is a
	/// live container. Skipped when absent so non-container responses stay
	/// byte-identical to the pre-live shape.
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "live")]
	pub Live:Option<LiveProgress>,
}
