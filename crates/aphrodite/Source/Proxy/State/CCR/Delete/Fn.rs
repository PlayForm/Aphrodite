//! Delete a CCR entry on a blocking thread.
//!
//! Reports whether the store accepted the delete; the `/ccr/delete` handler
//! turns that into its status code rather than assuming success.

use std::sync::Arc;

use headroom_core::ccr::CcrStore;

/// Wrapper for `ccr.del()` on a blocking thread.
pub(crate) async fn Fn(ccr:&Arc<dyn CcrStore>, hash:&str) -> bool {
	let ccr = ccr.clone();
	let hash = hash.to_owned();
	tokio::task::spawn_blocking(move || ccr.del(&hash)).await.unwrap_or(false)
}
