//! Why the context engine archived nothing.

/// The refusal a gate produced, in the order the gates run.
///
/// It is a value because the gates are an ordered policy about the index, not a
/// decomposition of one: each rule either refuses or gets out of the way, and the
/// FIRST refusal is the one worth reporting. Carrying it as an enum means the
/// interpreter reports exactly one reason and the order is visible as data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enum {
	/// The engine is off, or the threshold is `0` (never) or `100+` (off).
	Disabled,
	/// Fewer archived turns than the configured minimum.
	BelowMinimum,
	/// The index's fill ratio is below the configured threshold.
	BelowThreshold,
	/// The middle slice would be shorter than the minimum worth compacting.
	Small,
	/// The protect bounds do not describe a slice of this index at all - a
	/// configuration pair whose head reaches past the tail. Distinct from
	/// `Small` because the fix is different: the bounds are wrong, not
	/// the history.
	OutOfRange,
	/// The middle turns could not be packed. Unreachable in practice -
	/// serialising a `serde_json::Value` cannot fail - but named rather than
	/// panicking, because a refusal is always cheaper than a crash.
	Unpackable,
}

impl Enum {
	/// The reason string the DEV-ONLY `engine_skip` line records.
	pub(crate) const fn Text(&self) -> &'static str {
		match self {
			Enum::Disabled => "engine_off_or_threshold_gate",
			Enum::BelowMinimum => "below_min_msgs",
			Enum::BelowThreshold => "fill_below_threshold",
			Enum::Small => "middle_too_small",
			Enum::OutOfRange => "middle_out_of_range",
			Enum::Unpackable => "unpackable",
		}
	}
}
