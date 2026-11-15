//! The compression path a tool call is predicted to take.

/// The compression path a tool call is predicted to take.
///
/// The config booleans and the tool name are not independent switches: they are
/// one ORDERED decision, and most of their combinations are meaningless (a
/// self-tool call cannot also be an essential one, and an essential tool never
/// reaches the chain-split test). Naming the outcomes makes that order explicit
/// and lets the renderer match on a value instead of re-testing the flags.
///
/// This is a label for the dev-only pre-tool log line, not a control decision -
/// the transform arms log the ACTUAL outcome, so a wrong prediction mislabels a
/// log line and nothing else.
pub(crate) enum Enum {
	/// An essential tool (`skill_view`, `read_file`, ...): never compressed.
	Essential,
	/// One of Aphrodite's own tools, or headroom's: never compressed.
	SelfTool,
	/// `auto_expand` is on, so the call is expanded rather than compressed.
	AutoExpand,
	/// A chained terminal command, with chain splitting enabled.
	ChainSplit,
	/// A live-container session.
	LiveContainer,
	/// Everything else: compress when the threshold says so, else pass through.
	CompressOrPassthrough,
}
