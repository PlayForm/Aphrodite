//! The line-statistics result.

/// What the first pass over the content yields: the leading excerpt, the two
/// tallies and the first diagnostic line.
///
/// A struct rather than a tuple because the four values are inserted into the
/// variable map under four unrelated names, and a positional return made it too
/// easy to swap `ErrorCount` and `WarningCount` - a swap that renders a
/// plausible but inverted preview.
pub(crate) struct Struct {
	/// The first ~110 characters of the content, newlines flattened, trimmed.
	pub(crate) FirstLine:String,
	/// Lines mentioning `error` (case-insensitive), in the whole content.
	pub(crate) ErrorCount:usize,
	/// Lines mentioning `warning` (case-insensitive), in the whole content.
	pub(crate) WarningCount:usize,
	/// The first line mentioning `error` or `failed`, trimmed and capped.
	pub(crate) Message:String,
}
