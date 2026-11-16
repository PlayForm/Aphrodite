//! Reassemble a live container from its stored chunks.

use std::ops::ControlFlow;

use crate::{Proxy::State::Struct::Struct as AppState, Retrieve::Live::Container};

/// Look up a live container's metadata envelope and reassemble its content.
///
/// Returns `None` (the caller then serves the plain resolved content) when the entry is not a
/// live container or its envelope is missing/malformed - warn + degrade, never panic. When the
/// entry IS a live container, the content is reassembled from the container's own chunks (its
/// authoritative store); a missing chunk degrades to the assembled prefix so far.
///
/// The "is this a container" question is a value, not a pair of tests at the call site:
/// `Enable` is the envelope's own verdict and `Count` is what there is to reassemble, so a
/// degenerate envelope (a container claiming no chunks) answers `None` here instead of running
/// a loop over nothing and then substituting the plain entry.
///
/// The reassembled string is returned BY VALUE, so the caller takes it rather than copying it
/// out of a borrowed pair. An empty reassembly means the container held nothing usable, and the
/// plain resolved entry is then the honest answer - `None` is never "the container was empty".
pub(crate) fn Fn(State:&AppState, hash:&str, Resolved:&str) -> Option<(Container::Struct::Struct, String)> {
	let Key = crate::Hook::Live::Key::Meta::Fn::Fn(hash);
	// `get` is `&mut self` on the cache (a hit promotes recency) - the same reason the
	// handler's inline lookup binds a mutable guard.
	let mut Map = match State.InlineCCR.lock() {
		Ok(Map) => Map,
		Err(Poisoned) => Poisoned.into_inner(),
	};
	let JSON = Map.get(&Key)?.clone();
	let Envelope:Container::Struct::Struct = match serde_json::from_str(&JSON) {
		Ok(Envelope) => Envelope,
		Err(Error) => {
			tracing::warn!(hash, error = %Error, "retrieve: live container envelope unparseable; serving as plain entry");

			return None;
		},
	};

	// Not a live container, or a degenerate envelope - serve plainly.
	let Count = (Envelope.Enable && Envelope.Count > 0).then_some(Envelope.Count)?;

	let mut Assembled = String::new();

	let _ = (0..Count).try_fold((), |(), Index| {
		match Map.get(&crate::Hook::Live::Key::Chunk::Fn::Fn(hash, Index)) {
			Some(Chunk) => {
				Assembled.push_str(Chunk);

				ControlFlow::Continue(())
			},
			None => {
				tracing::warn!(
					hash,
					Chunk = Index,
					"retrieve: live container chunk missing; serving assembled prefix"
				);

				ControlFlow::Break(())
			},
		}
	});

	let Content = (!Assembled.is_empty())
		.then_some(Assembled)
		.unwrap_or_else(|| Resolved.to_string());

	Some((Envelope, Content))
}
