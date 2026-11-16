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
	Prefetch::{Arrival::Struct::LiveArrival, Prepared::Struct::Prepared, Record::Enum::Record},
	State::{Marker::Struct::MarkerEntry, Struct::AphroditeState},
};

/// Put the content in the store, record its marker and file entry, report it loaded.
pub(crate) fn Fn(State:&mut AphroditeState, Prepared:Prepared, ArrivedAtUnix:u64) -> Record {
	let Prepared { Path, Content, Hash, Type, Preview, Size } = Prepared;

	State.InlineStorePut(Hash.as_str().to_string(), Content);

	let _ = (!State.InlineStoreContains(Hash.as_str())).then(|| {
		tracing::warn!(
			Path = %Path,
			Hash = Hash.as_str(),
			"prefetch: entry was evicted by the inline store budget immediately after it was stored"
		);
	});

	State.RecordMarker(MarkerEntry {
		hash:Hash.as_str().to_string(),
		CCRType:Type.clone(),
		Size,
		Preview:Preview.clone(),
		Turn:State.TurnCounter,
		Center:None,
		meta:Some({
			let mut Meta = std::collections::HashMap::new();
			Meta.insert("path".to_string(), Path.clone());
			// Live-container mode (pre-Rust `_hooks/live.py`): the schedule entry
			// carries container semantics - arrival timestamp, byte progress, ETA - so
			// the prefetch_status surface can expose them. Off by default: meta keeps
			// only `path` (byte-identical current behavior).
			if State.LiveContainer {
				let Arrival = LiveArrival::Arrived(Size as u64, ArrivedAtUnix);

				Meta.insert("live".to_string(), "1".to_string());
				Meta.insert("live_arrived_at".to_string(), Arrival.ArrivedAtUnix.to_string());
				Meta.insert("live_bytes_arrived".to_string(), Arrival.BytesArrived.to_string());
				Meta.insert("live_eta_s".to_string(), Arrival.EtaS.to_string());
			}
			Meta
		}),
	});

	State.RecordFile(Path.clone(), "prefetch".to_string());

	Record::Loaded { Path, Hash, Type, Size, Preview }
}
