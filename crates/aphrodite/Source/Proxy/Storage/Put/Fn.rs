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
///
/// The blocking job hands back the allocation it wrote, and the evidence is built
/// from that. The old shape allocated the content twice - once to move into the
/// closure, then again from the caller's borrow to build the receipt - so every
/// stored payload was copied for a value the job already owned.
pub(crate) async fn Fn(ccr:&Arc<dyn CcrStore>, Hash:&str, Content:&str) -> Option<StoreEvidence> {
	let ccr = ccr.clone();
	let Hash = Hash.to_owned();
	let Content = Content.to_owned();

	let Written = tokio::task::spawn_blocking(move || {
		let Landed = ccr.put_checked(&Hash, &Content);

		(Landed, Hash, Content)
	})
	.await
	.ok()?;

	match Written {
		(true, Hash, Content) => Some(StoreEvidence { Hash, Content }),
		(false, ..) => None,
	}
}
