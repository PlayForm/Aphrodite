//! Store content as a live container and emit the instant marker.

use super::super::Marker;
use crate::{
	Hook::Live::{
		Chunk::Fn::Fn as ChunkOnCharBoundaries,
		Const::LiveChunkSize,
		Key::Fn::{LiveChunkKey, LiveMetaKey},
	},
	State::{Marker::Struct::MarkerEntry, Struct::AphroditeState},
};

/// Live-container transform (pre-Rust port of `_wrap_as_live_container`):
/// store the content chunked with a metadata envelope, record a `Live`
/// marker entry, and return the INSTANT marker payload with a progress
/// annotation. Call only when `state.live_container` is on and the content
/// cleared the compress threshold (and auto-expand is not taking over).
///
/// Two stores, on purpose: the plain hash keeps the lossless round-trip
/// contract for `Resolve::Expand::Fn`, and the chunk entries give the retrieve
/// path something to serve progressively. Dropping either one breaks a
/// different consumer.
pub(crate) fn Fn(State:&mut AphroditeState, Content:&str, TypeStr:&str) -> serde_json::Value {
	let hash = headroom_core::ccr::compute_key(Content.as_bytes());
	let Total = Content.len();
	let Chunks = ChunkOnCharBoundaries(Content, LiveChunkSize);
	let n = Chunks.len();

	// Round-trip entry: the plain hash resolves to the exact original via
	// `Resolve::Expand::Fn` (and is what a final full retrieve serves).
	State.InlineStorePut(hash.clone(), Content.to_string());
	// Container entries: per-chunk content + the metadata envelope.
	// `BytesArrived` starts at 0 - the marker is instant, delivery happens
	// through the retrieve path, which advances the counter.
	for (i, Chunk) in Chunks.iter().enumerate() {
		State.InlineStorePut(LiveChunkKey(&hash, i), (*Chunk).to_string());
	}
	let meta = serde_json::json!({
		"live": true,
		"type": TypeStr,
		"size": Total,
		"chunks": n,
		"chunk_size": LiveChunkSize,
		"bytes_arrived": 0,
	});
	State.InlineStorePut(LiveMetaKey(&hash), meta.to_string());

	let Preview = crate::Preview::Builder::Entry::Fn(TypeStr, Content);
	let Marker = Marker::Fn::Fn(&hash, Total, n, &Preview);

	State.RecordMarker(MarkerEntry {
		hash:hash.clone(),
		CCRType:"live".to_string(),
		Size:Total,
		Preview:Preview.clone(),
		Turn:State.TurnCounter,
		Center:None,
		meta:None,
	});

	// DEV-ONLY debug metrics (crate::debug): count the live-container wrap.
	crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::LiveContainers);
	serde_json::json!({
		"status": "ok",
		"compressed": true,
		"live": true,
		"hash": hash,
		"type": "live",
		"size": Total,
		"chunks": n,
		"bytes_arrived": 0,
		"preview": Preview,
		"marker": Marker,
	})
}
