//! Which upstream-error counter a status feeds.
//!
//! Two counters exist, so the status has to be reduced to the two classes that
//! increment them; everything else is `Other` and counted as nothing. The
//! reduction is a value so the counter increment is a `match` on a domain enum
//! rather than two ordered guard arms on a number.

/// The status classes the proxy counts.
pub(crate) enum Enum {
	/// 5xx - the upstream failed.
	ServerError,
	/// 4xx - the upstream rejected the request.
	ClientError,
	/// Anything else, including every success.
	Other,
}
