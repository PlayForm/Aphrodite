//! Why a hook transformed nothing.

/// The reason a hook deliberately passed content through.
///
/// One vocabulary for both transform paths: a passthrough is a decision, and the
/// reason is what the payload reports and what the trace records, so the two can
/// never disagree about why nothing happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enum {
	/// There was no content at all.
	Empty,
	/// The tool returns content the agent acts on, so it is never replaced.
	EssentialTool,
	/// The tool belongs to Aphrodite itself; compressing it would loop.
	SelfTool,
	/// The content is smaller than the configured threshold.
	BelowThreshold,
}

impl Enum {
	/// The `reason` string the payload carries.
	///
	/// A wire contract: the bridge and the plugin's tests read these exact
	/// spellings, so they are matched here rather than formatted from the
	/// variant name.
	pub(crate) const fn Text(&self) -> &'static str {
		match self {
			Enum::Empty => "empty",
			Enum::EssentialTool => "essential_tool",
			Enum::SelfTool => "self_tool",
			Enum::BelowThreshold => "below_threshold",
		}
	}
}
