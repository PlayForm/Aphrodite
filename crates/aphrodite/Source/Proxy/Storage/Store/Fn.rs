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
pub(crate) async fn Fn(ccr:&Arc<dyn CcrStore>, Hash:&str, Content:&str) -> StoreOutcome {
	let Readable = {
		let ccr = ccr.clone();
		let Hash = Hash.to_owned();

		tokio::task::spawn_blocking(move || ccr.get(&Hash)).await.unwrap_or(None)
	};

	match Readable {
		Some(Stored) => StoreOutcome::Cached(StoreEvidence { Hash:Hash.to_owned(), Content:Stored }),
		None => {
			let Written = {
				let ccr = ccr.clone();
				let Hash = Hash.to_owned();
				let Content = Content.to_owned();

				tokio::task::spawn_blocking(move || ccr.put_checked(&Hash, &Content))
					.await
					.unwrap_or(false)
			};

			if Written {
				StoreOutcome::Stored(StoreEvidence { Hash:Hash.to_owned(), Content:Content.to_owned() })
			} else {
				StoreOutcome::Failed
			}
		},
	}
}
