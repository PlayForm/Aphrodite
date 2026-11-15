//! The `offset`/`limit` compatibility adapter.

use std::num::NonZeroUsize;

use super::super::{Enum::Enum, MaximumWindowLines::Const::Const as MaximumWindowLines};

/// Interpret a wire `(Offset, Limit)` pair as a retrieval mode.
///
/// This is the one place the `limit: 0` sentinel is understood, and it exists
/// only for callers that still speak the wire pair: the handler parses the
/// request through it, and [`super::super::Fn::Fn`] wraps it for the integration
/// tests that pin the byte round-trip. Everything downstream of this function
/// takes a [`Enum`] and cannot see a zero limit at all.
///
/// The pair is `(0, 0)` - no offset, no limit - and nothing else. Every other
/// combination is a window, INCLUDING a non-zero offset with `limit: 0`: that
/// request asks for the document's tail from `Offset`, so it is `Window {
/// Offset, Limit: None }` and still carries its `[lines a-b/total]` annotation.
/// Mapping it to `Exact` would return the whole document where the caller asked
/// for a tail, which is a silent behaviour change in the one field the
/// round-trip contract does not cover.
///
/// A non-zero limit is clamped here rather than in the slicer, so `Window` can
/// only be built with a limit the server is willing to serve.
pub fn Fn(Offset:usize, Limit:usize) -> Enum {
	let Limit = NonZeroUsize::new(Limit.min(MaximumWindowLines));

	match (Offset, Limit) {
		(0, None) => Enum::Exact,
		(Offset, Limit) => Enum::Window { Offset, Limit },
	}
}
