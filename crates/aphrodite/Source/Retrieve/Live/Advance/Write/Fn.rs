//! Persist a live container's advanced progress.

use crate::{Proxy::State::Struct::Struct as AppState, Retrieve::Live::Container::Struct::Struct as LiveContainer};

/// Write the advanced `BytesArrived` back into the container's envelope.
///
/// Best effort by design: the envelope is a progress record beside the content, so a
/// poisoned lock or a serialisation hiccup costs the next retrieve its starting point and
/// nothing else. Both are `let ... else` returns rather than error paths because no caller
/// can act on either - and a retrieve must not fail because its bookkeeping did.
pub(crate) fn Fn(State:&AppState, hash:&str, Container:&LiveContainer, BytesArrived:usize) {
	let Ok(mut Map) = State.InlineCCR.lock() else {
		return;
	};

	let Updated = LiveContainer { BytesArrived, ..Container.clone() };
	let Ok(Encoded) = serde_json::to_string(&Updated) else {
		return;
	};

	Map.put(crate::Hook::Live::Key::Fn::LiveMetaKey(hash), Encoded);
}
