//! Store content as a live container and emit the instant marker.

use super::super::{Marker, Object::Install::Fn::Fn as InstallObject};
use crate::State::{Marker::Struct::MarkerEntry, Struct::AphroditeState};

/// Live-container transform (pre-Rust port of `_wrap_as_live_container`):
/// store the content as one container object, record a `Live` marker entry, and
/// return the INSTANT marker payload with a progress annotation. Call only when
/// `state.live_container` is on and the content cleared the compress threshold
/// (and auto-expand is not taking over).
///
/// The store layout is `Object::Install`'s business: this leaf asks for the
/// object and then describes it - marker, catalog entry, payload - all from the
/// same value, so the marker's chunk count and total size are the ones that were
/// stored rather than a second reading of the content.
pub(crate) fn Fn(State:&mut AphroditeState, Content:&str, TypeStr:&str) -> serde_json::Value {
	let Span = tracing::info_span!(
		"hook.live_container",
		ccr_type = %TypeStr,
		content_bytes = Content.len(),
		chunks = tracing::field::Empty,
	);
	let _Entered = Span.enter();

	let Object = InstallObject(State, Content, TypeStr);

	let Preview = crate::Preview::Builder::Entry::Fn(TypeStr, Content);
	let Marker = Marker::Fn::Fn(&Object.Hash, Object.Total, Object.Chunks, &Preview);

	State.RecordMarker(MarkerEntry {
		hash:Object.Hash.clone(),
		CCRType:"live".to_string(),
		Size:Object.Total,
		Preview:Preview.clone(),
		Turn:State.TurnCounter,
		Center:None,
		meta:None,
	});

	// DEV-ONLY debug metrics (crate::debug): count the live-container wrap.
	crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::LiveContainers);
	Span.record("chunks", Object.Chunks);
	let Payload = serde_json::json!({
		"status": "ok",
		"compressed": true,
		"live": true,
		"hash": Object.Hash,
		"type": "live",
		"size": Object.Total,
		"chunks": Object.Chunks,
		"bytes_arrived": 0,
		"preview": Preview,
		"marker": Marker,
	});
	Payload
}
