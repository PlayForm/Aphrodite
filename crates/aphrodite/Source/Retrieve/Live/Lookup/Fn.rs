//! Reassemble a live container from its stored chunks.

use crate::{Proxy::State::Struct::Struct as AppState, Retrieve::Live::Container::Struct::Struct as LiveContainerMeta};

/// Look up a live container's metadata envelope and reassemble its content.
///
/// Returns `None` (the caller then serves the plain resolved content) when the entry is not a
/// live container or its envelope is missing/malformed - warn + degrade, never panic. When the
/// entry IS a live container, the content is reassembled from the container's own chunks (its
/// authoritative store); a missing chunk degrades to the assembled prefix so far.
///
/// The "is this a container" question is a value, not a pair of tests at the call site: `Live`
/// is the envelope's own verdict and `Chunks` is what there is to reassemble, so a degenerate
/// envelope (a container claiming no chunks) answers `None` here instead of running a loop over
/// nothing and then substituting the plain entry.
///
/// The reassembled string is returned BY VALUE, so the caller takes it rather than copying it
/// out of a borrowed pair. An empty reassembly means the container held nothing usable, and the
/// plain resolved entry is then the honest answer.
pub(crate) fn Fn(State:&AppState, hash:&str, Resolved:&str) -> Option<(LiveContainerMeta, String)> {
	let MetaKey = crate::Hook::Live::Key::Fn::LiveMetaKey(hash);
	// `get` is `&mut self` on the cache (a hit promotes recency) - the same reason the
	// handler's inline lookup binds a mutable guard.
	let mut Map = match State.InlineCCR.lock() {
		Ok(Map) => Map,
		Err(Poisoned) => Poisoned.into_inner(),
	};
	let MetaJSON = Map.get(&MetaKey)?.clone();
	let Meta:LiveContainerMeta = match serde_json::from_str(&MetaJSON) {
		Ok(Meta) => Meta,
		Err(Error) => {
			tracing::warn!(hash, error = %Error, "retrieve: live container envelope unparseable; serving as plain entry");

			return None;
		},
	};

	// Not a live container, or a degenerate envelope - serve plainly.
	let Chunks = (Meta.Live && Meta.Chunks > 0).then_some(Meta.Chunks)?;

	let mut Assembled = String::new();

	for Index in 0..Chunks {
		let Some(Chunk) = Map.get(&crate::Hook::Live::Key::Fn::LiveChunkKey(hash, Index)) else {
			tracing::warn!(
				hash,
				Chunk = Index,
				"retrieve: live container chunk missing; serving assembled prefix"
			);

			break;
		};

		Assembled.push_str(Chunk);
	}

	let Content = if Assembled.is_empty() { Resolved.to_string() } else { Assembled };

	Some((Meta, Content))
}
