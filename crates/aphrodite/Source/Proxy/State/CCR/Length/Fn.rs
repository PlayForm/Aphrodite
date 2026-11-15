//! Entry count of the CCR backend, on a blocking thread.
//!
//! `len()` is O(1) per backend contract, but it still crosses the sync/async
//! boundary, so it is spawned like every other store call rather than called
//! inline from `/stats`.

use std::sync::Arc;

use headroom_core::ccr::CcrStore;

/// Wrapper for `ccr.len()` on a blocking thread.
pub(crate) async fn Fn(ccr:&Arc<dyn CcrStore>) -> usize {
	let ccr = ccr.clone();
	tokio::task::spawn_blocking(move || ccr.len()).await.unwrap_or(0)
}

// ── State ──────────────────────────────────────────────────────────
