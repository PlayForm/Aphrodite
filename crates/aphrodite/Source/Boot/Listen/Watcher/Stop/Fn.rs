//! Stopping the config watcher at shutdown.

use crate::Boot::Listen::Watcher::Outcome;

/// Stop a live watch, if there is one.
///
/// The watcher task lives as long as the process and holds the `notify` watcher that
/// feeds it, so it cannot end on its own: its channel closes only when the watcher is
/// dropped, and the watcher is dropped only when the task ends. Stopping it explicitly
/// is what makes the shutdown sequence complete - the alternative was a task whose
/// handle was discarded at startup and which died whenever the process happened to
/// exit.
///
/// An `Inactive` outcome has nothing to stop: it is the report that no task was ever
/// spawned.
pub fn Fn(Outcome:Outcome::Enum::Enum) {
	let Outcome::Enum::Enum::Watching(Handle) = Outcome else {
		return;
	};

	Handle.abort();

	tracing::debug!("config file watcher stopped");
}
