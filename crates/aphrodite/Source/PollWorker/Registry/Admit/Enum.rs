//! What an admission did.

/// The outcome of admitting a task.
///
/// The eviction is reported rather than logged, so the policy stays a policy and
/// the caller decides what an eviction means to say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Admit {
	/// Admitted with room to spare.
	Admitted,
	/// Admitted after evicting this task (its command line).
	Evicted { Label:String },
	/// Already tracked while running - nothing changed.
	Duplicate,
}
