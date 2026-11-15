//! Store a CCR entry on a blocking thread, reporting whether it landed.
//!
//! The boolean is the contract: a caller that ignores it and replaces the
//! original content with a marker anyway ships a marker whose hash resolves to
//! nothing - permanent loss of content the client never received.

use std::sync::Arc;

use headroom_core::ccr::CcrStore;

/// Wrapper for `ccr.put()` on a blocking thread.
/// Store `Content` under `hash` in the CCR backend. Returns whether the
/// store actually succeeded (F4) - a caller that discards this and replaces
/// the original content with a marker anyway ships a marker whose hash
/// resolves to nothing the moment the store is full/locked/panics, which is
/// permanent data loss (the original content never reached the client).
pub(crate) async fn Fn(ccr:&Arc<dyn CcrStore>, hash:&str, Content:&str) -> bool {
	let ccr = ccr.clone();
	let hash = hash.to_owned();
	let Content = Content.to_owned();
	tokio::task::spawn_blocking(move || ccr.put_checked(&hash, &Content))
		.await
		.unwrap_or(false)
}
