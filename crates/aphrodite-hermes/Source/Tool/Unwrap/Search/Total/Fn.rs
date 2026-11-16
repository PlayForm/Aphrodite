//! The count-only search payload.

/// Render the REAL total, and the truncated flag when the search hit its cap.
///
/// This is the payload itself when there is no per-match content to count, typed
/// `text` so the generic preview arm renders `0 total` / `N total (truncated)`
/// instead of an unreadable `[search:1L]`.
pub(crate) fn Fn(Count:u64, Truncated:bool) -> String {
	match Truncated {
		true => format!("{Count} total (truncated)"),
		false => format!("{Count} total"),
	}
}
