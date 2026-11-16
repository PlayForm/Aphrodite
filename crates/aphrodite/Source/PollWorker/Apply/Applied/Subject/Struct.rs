//! Which task a poll applied to, and what it is called.

use crate::State::Identity::Command::Struct::Struct as Command;

/// The task one poll applied to.
///
/// Three facts travel together because every consumer of an applied poll needs all
/// three and they are read from the task under the registry's borrow: the position
/// (so the completion can be recorded without a second search), the command identity
/// (so the log line and the nudge name the task), and the command line (so the
/// marker's centre label can name it). Splitting them into separate arguments is how
/// an index and an identity get crossed.
#[derive(Debug, Clone)]
pub(crate) struct Subject {
	/// Position of the task in the registry.
	pub Index:usize,
	/// Command identity of that task.
	pub ID:Command,
	/// The command line, for the marker's centre label.
	pub Command:String,
}
