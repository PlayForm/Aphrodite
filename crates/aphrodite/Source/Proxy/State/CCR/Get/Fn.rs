//! Read a CCR entry on a blocking thread.
//!
//! The store backends are synchronous (SQLite above all), so every call is
//! moved off the reactor; a direct call from a handler would block the whole
//! runtime thread. Returns `None` when the store is absent or the hash is
//! unknown - the caller decides what a miss means.

use std::sync::Arc;

use headroom_core::ccr::CcrStore;

/// Wrapper for `ccr.get()` on a blocking thread.
pub(crate) async fn Fn(ccr:&Arc<dyn CcrStore>, hash:&str) -> Option<String> {
	let ccr = ccr.clone();
	let hash = hash.to_owned();
	tokio::task::spawn_blocking(move || ccr.get(&hash)).await.unwrap_or(None)
}
