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
///
/// Each variant names the log label it renders to (`Render::Fn::Fn`), because an
/// operator greps those strings: they are pinned and do not follow a rename.
pub(crate) enum Enum {
	/// An essential tool (`skill_view`, `read_file`, ...): never compressed.
	/// Renders as `passthrough_essential`.
	Essential,
	/// One of Aphrodite's own tools, or headroom's: never compressed. Two words
	/// on purpose - `Self` is a keyword and `Own` alone does not say what is
	/// owned. Renders as `passthrough_self`.
	SelfTool,
	/// `[compression] auto_expand` is on, so the call is expanded rather than
	/// compressed. The word is the config key's own noun, so the flag and the
	/// route cannot drift apart. Renders as `auto_expand`.
	AutoExpand,
	/// A chained terminal command, with `chain_split_enabled` on. The word is
	/// the config key's own noun. Renders as `chain_split`.
	ChainSplit,
	/// A live-container session. The word is the state field's own noun.
	/// Renders as `live_container`.
	LiveContainer,
	/// Everything else: the compression threshold decides - compress when the
	/// payload exceeds it, pass through when it does not. Named for the thing
	/// that decides instead of for both of its outcomes. Renders as
	/// `compress_or_passthrough`.
	Threshold,
}
