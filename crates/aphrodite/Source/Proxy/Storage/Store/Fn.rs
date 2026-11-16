//! Ensure content is readable under a hash, and prove it.
//!
//! Reads first, because the common case in a long session is content that was
//! already stored - the same tool output compressed twice - and a `put` of
//! identical bytes is wasted work on a SQLite backend. A read that finds the
//! content is evidence enough: the marker it produces resolves.
//!
//! F4: a failed `put` must never be followed by swapping the response for an
//! unresolvable marker, so `Failed` carries no evidence at all and the caller
//! has nothing to render a marker from.

use std::sync::Arc;

use headroom_core::ccr::CcrStore;

use crate::Proxy::Storage::{Evidence::Struct::Struct as StoreEvidence, Store::Outcome::Enum::Enum as StoreOutcome};

/// Ensure `Content` is readable under `Hash`.
///
/// Every blocking job hands back the allocations it was given, and each outcome is
/// built from those: a payload is copied into the job exactly once and never
/// copied again to reconstruct a receipt. The old shape cloned the content for the
/// closure and then cloned it a second time from the caller's borrow - a full copy
/// of every stored payload for a value the job already owned.
pub(crate) async fn Fn(ccr:&Arc<dyn CcrStore>, Hash:&str, Content:&str) -> StoreOutcome {
	let Hash = Hash.to_owned();
	let Content = Content.to_owned();

	// The read goes first because the common case in a long session is content that
	// was already stored - the same tool output compressed twice - and a `put` of
	// identical bytes is wasted work on a SQLite backend. The job hands the hash
	// back so the write branch does not need a second copy of the payload.
	let Read = {
		let ccr = ccr.clone();
		let Hash = Hash.clone();

		tokio::task::spawn_blocking(move || {
			let Stored = ccr.get(&Hash);

			(Hash, Stored)
		})
		.await
	};

	match Read {
		// A read that finds the content is evidence enough: the marker it produces
		// resolves. Both halves are the job's own allocations.
		Ok((Hash, Some(Stored))) => StoreOutcome::Cached(StoreEvidence { Hash, Content:Stored }),
		// Not readable, or the job panicked: either way the content is not known to
		// be there, so the write is attempted.
		Ok((_, None)) | Err(_) => {
			let Written = {
				let ccr = ccr.clone();

				tokio::task::spawn_blocking(move || {
					let Landed = ccr.put_checked(&Hash, &Content);

					(Landed, Hash, Content)
				})
				.await
			};

			match Written {
				Ok((true, Hash, Content)) => StoreOutcome::Stored(StoreEvidence { Hash, Content }),
				// F4: a failed `put` must never be followed by swapping the response
				// for an unresolvable marker, so `Failed` carries no evidence at all.
				Ok((false, ..)) | Err(_) => StoreOutcome::Failed,
			}
		},
	}
}
