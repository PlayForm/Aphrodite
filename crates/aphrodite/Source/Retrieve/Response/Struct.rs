//! What the retrieve endpoint answers with.

use serde::Serialize;

use crate::Retrieve::Progress::Struct::Struct as LiveProgress;

/// Response body for `/retrieve`. `Source` reports which backend served the
/// content (e.g. `"inline_ccr"`, `"ccr"`, `"none"`) for observability.
#[derive(Debug, Serialize)]
pub struct Struct {
	/// `true` when `Content` carries a payload (a full document or a window of
	/// one); `false` on every error shape.
	pub Found:bool,
	/// The resolved payload, or `None` when the lookup failed.
	pub Content:Option<String>,
	/// Which backend served it (`InlineCCR`, `ccr`, `none`).
	pub Source:String,
	/// Human-readable failure reason; `None` on success.
	pub error:Option<String>,
	/// `true` when `Content` is a partial window of a larger stored
	/// document - because of `Offset`/`Limit`, or because an explicit `Limit`
	/// hit the 10,000-line server cap (02-F5). `limit: 0` requests the FULL
	/// document (round-trip contract: retrieval must return the exact original
	/// bytes so the body hashes to its own marker hash). A `[lines a-b/total]`
	/// header is also prepended to `Content` in that case; this field lets a
	/// caller detect truncation without parsing it.
	pub Truncated:bool,
	/// Live-container progress; present only when the resolved entry is a
	/// live container. Skipped when absent so non-container responses stay
	/// byte-identical to the pre-live shape.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub Live:Option<LiveProgress>,
}
