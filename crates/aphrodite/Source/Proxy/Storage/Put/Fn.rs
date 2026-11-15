//! Write content under a hash, on a blocking thread, reporting evidence.
//!
//! The store backends are synchronous (SQLite above all), so every call is
//! moved off the reactor; a direct call from a handler would block the whole
//! runtime thread for the duration of a disk write.
//!
//! This is the write-only half of storage: the endpoints that create an entry
//! on request (`/ccr/create`, the tool relay) already know the content is new,
//! so a read before the write would be a wasted round-trip. The compression
//! path uses `Storage::Store` instead, because there the content usually IS
//! already there.

use std::sync::Arc;

use headroom_core::ccr::CcrStore;

use crate::Proxy::Storage::Evidence::Struct::Struct as StoreEvidence;

/// Store `Content` under `Hash`; `None` means the backend refused the write.
pub(crate) async fn Fn(ccr:&Arc<dyn CcrStore>, Hash:&str, Content:&str) -> Option<StoreEvidence> {
	let ccr = ccr.clone();
	let hash = Hash.to_owned();
	let Payload = Content.to_owned();

	tokio::task::spawn_blocking(move || ccr.put_checked(&hash, &Payload))
		.await
		.unwrap_or(false)
		.then(|| StoreEvidence { Hash:Hash.to_owned(), Content:Content.to_owned() })
}
