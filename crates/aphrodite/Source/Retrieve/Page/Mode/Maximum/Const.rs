//! The server cap on an explicit line window.

/// Maximum lines an EXPLICIT `Limit` may return (02-F5).
///
/// A nonzero `Limit` is clamped to this, so a caller asking for a million lines
/// gets ten thousand and an honest `truncated: true` rather than an
/// unbounded response. `Exact` never consults it: the full document is the
/// content-addressing contract and is returned whatever its length, which is
/// exactly why the two requests are now different values rather than two
/// meanings of one `usize`.
pub(crate) const Const:usize = 10_000;
