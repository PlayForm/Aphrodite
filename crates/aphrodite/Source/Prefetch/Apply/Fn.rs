//! Store a prepared file and record what the session now knows about it.
//!
//! No filesystem access happens here, and no classification either: `Prepare` did both
//! before this call took the session. What remains is the mutation - content into the
//! inline store, a marker into the catalog, the path into the file list - which is why
//! the borrowed window is short and why this function cannot fail.
//!
//! Order is behaviour. The content is stored before the marker that names it, and the
//! marker before the file entry: an interruption leaves content with no marker, or a
//! marker with no "you already have this" hint - never a marker pointing at content the
//! store does not hold. A store entry the byte budget evicts immediately after the put
//! is warned about, because a listed hash that no longer resolves is indistinguishable
//! from a retrieve bug.

use crate::{
	Prefetch::{Arrival::Struct::Struct as Arrival, Prepared::Struct::Prepared, Record::Enum::Record},
	State::{Marker::Struct::Struct as Entry, Struct::AphroditeState},
};

/// Put the content in the store, record its marker and file entry, report it loaded.
pub(crate) fn Fn(State:&mut AphroditeState, Prepared:Prepared, ArrivedAtUnix:u64) -> Record {
	let Prepared { Path, Content, Hash, Type, Preview, Size } = Prepared;

	State.Put(Hash.as_str().to_string(), Content);

	// The store decides whether the content is still there: an entry the budget
	// evicted immediately is a listed hash that cannot resolve, so the hash it lost
	// is what the warning reports.
	(!State.Contains(Hash.as_str()))
		.then_some(Hash.as_str())
		.into_iter()
		.for_each(|Hash| {
			tracing::warn!(
				Path = %Path,
				Hash = Hash,
				"prefetch: entry was evicted by the inline store budget immediately after it was stored"
			);
		});

	State.RecordMarker(Entry {
		hash:Hash.as_str().to_string(),
		CCRType:Type.clone(),
		Size,
		Preview:Preview.clone(),
		Turn:State.TurnCounter,
		Center:None,
		meta:Some({
			// The base entry is the one item the map always holds; the live fields are
			// an optional tail fed in beside it, so the flag decides what the collection
			// contains instead of gating inserts into it.
			let Base = std::iter::once(("path".to_string(), Path.clone()));

			// Live-container mode (pre-Rust `_hooks/live.py`): the schedule entry
			// carries container semantics - arrival timestamp, byte progress, ETA - so
			// the prefetch_status surface can expose them. Off by default: meta keeps
			// only `path` (byte-identical current behavior).
			let Live = State
				.LiveContainer
				.then(|| Arrival::Arrived(Size as u64, ArrivedAtUnix))
				.into_iter()
				.flat_map(|Arrival| {
					[
						("live".to_string(), "1".to_string()),
						("live_arrived_at".to_string(), Arrival.ArrivedAtUnix.to_string()),
						("live_bytes_arrived".to_string(), Arrival.BytesArrived.to_string()),
						("live_eta_s".to_string(), Arrival.EtaS.to_string()),
					]
				});

			Base.chain(Live).collect()
		}),
	});

	State.RecordFile(Path.clone(), "prefetch".to_string());

	Record::Loaded { Path, Hash, Type, Size, Preview }
}
