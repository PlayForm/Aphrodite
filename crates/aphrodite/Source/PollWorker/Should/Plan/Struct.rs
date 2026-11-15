//! The identity and the label a backgrounded command gets.
//!
//! Two identities, deliberately distinct: `Identity` is the *command* identity, a
//! CCR hash of the command line, which is the deduplication key - so two
//! simultaneous runs of one command share it, on purpose. `Label` is the short
//! summary the marker and the nudges show. Neither is an execution identity: that
//! one is learned from the first poll that names the process and lives on the task
//! (`Task::Struct::ExecutionID`).

/// What a backgrounded command is identified by and shown as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BackgroundPlan {
	/// Command identity: CCR hash of the command line.
	pub Identity:String,
	/// Display label: the first 60 characters of the command line.
	pub Label:String,
}
