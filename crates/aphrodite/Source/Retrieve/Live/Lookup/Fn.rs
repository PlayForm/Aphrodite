//! Reassemble a live container from its stored chunks.

use crate::{Proxy::State::Struct::Struct as AppState, Retrieve::Live::Container::Struct::Struct as LiveContainerMeta};

/// Look up a live container's metadata envelope and reassemble its content.
///
/// Returns `None` (the caller then serves the plain resolved content) when
/// the entry is not a live container or its envelope is missing/malformed -
/// warn + degrade, never panic. When the entry IS a live container, the
/// content is reassembled from the container's own chunks (its
/// authoritative store); a missing chunk degrades to the assembled prefix
/// so far.
pub(crate) fn Fn(State:&AppState, hash:&str, Resolved:&str) -> Option<(LiveContainerMeta, String)> {
	let MetaKey = crate::Hook::Live::Key::Fn::LiveMetaKey(hash);
	// `get` is `&mut self` on LruCache (get promotes recency) - the same
	// reason the handler's inline_ccr lookup binds `mut map`.
	let mut Map = match State.InlineCCR.lock() {
		Ok(m) => m,
		Err(Poisoned) => Poisoned.into_inner(),
	};
	let MetaJSON = Map.get(&MetaKey)?.clone();
	let meta:LiveContainerMeta = match serde_json::from_str(&MetaJSON) {
		Ok(meta) => meta,
		Err(Error) => {
			tracing::warn!(
				hash,
				error = %Error,
				"retrieve: live container envelope unparseable; serving as plain entry"
			);
			return None;
		},
	};
	if !meta.Live || meta.Chunks == 0 {
		// Not a live container (or a degenerate envelope) - serve plainly.
		return None;
	}
	let mut Assembled = String::new();
	for i in 0..meta.Chunks {
		match Map.get(&crate::Hook::Live::Key::Fn::LiveChunkKey(hash, i)) {
			Some(Chunk) => Assembled.push_str(Chunk),
			None => {
				tracing::warn!(
					hash,
					Chunk = i,
					"retrieve: live container chunk missing; serving assembled prefix"
				);
				break;
			},
		}
	}
	let Content = if Assembled.is_empty() { Resolved.to_string() } else { Assembled };
	Some((meta, Content))
}
