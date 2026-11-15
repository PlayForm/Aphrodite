//! What a retrieve asks the window for.

use std::num::NonZeroUsize;

/// How much of a stored document a retrieve asks for.
///
/// Two contracts used to share one signature. An exact, content-addressed
/// retrieval must hand back the original bytes, because the returned body has to
/// hash to the marker's own hash; a human-oriented paginated display must hand
/// back a window with a `[lines a-b/total]` annotation. `Limit == 0` meant "the
/// full document", so nothing at the call site said which of the two it was
/// asking for, and the sentinel arithmetic lived inside the slicer.
///
/// `Exact` is the round-trip contract: the whole document, byte-identical,
/// trailing newline included, never annotated and never truncated.
///
/// `Window` is the display contract. `Offset` is the zero-based first line.
/// `Limit` is `None` for an unlimited tail - which is what the `limit: 0`
/// compatibility adapter produces for a NON-ZERO offset, so an
/// offset-with-unlimited-tail request keeps its annotation and its
/// `truncated: true` - or a non-zero line count already clamped to the server
/// cap. `None` is therefore not a second spelling of `Exact`: `Exact` has no
/// offset, and a window always renders its range.
pub enum Enum {
	/// The whole document, byte-identical. No annotation, never truncated.
	Exact,
	/// A line window: zero-based `Offset`, `Limit` of `None` for the tail.
	Window { Offset:usize, Limit:Option<NonZeroUsize> },
}
