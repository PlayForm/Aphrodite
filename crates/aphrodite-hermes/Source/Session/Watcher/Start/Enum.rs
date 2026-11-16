//! The result of one attempt to start the config watcher.

/// Whether the config watcher is running.
///
/// The spawn used to be a statement whose result was discarded (`let _ =
/// spawn(...)`), so a failed spawn - no thread, no notifier, no reload ever -
/// was invisible to the caller that had just asked for auto-reload. The two
/// states are now named, and the caller can report the one that needs
/// reporting.
///
/// `Running` carries nothing because the handle has exactly one owner: it is
/// parked in `Watcher::State::Running` at the moment the thread reports
/// readiness, so a caller cannot end up holding a second, unstoppable copy.
pub(crate) enum Enum {
	/// The thread is running, and `Watcher::State::Running` owns its handle
	/// (stop flag, wake sender and join handle).
	Running,
	/// It never started: no thread, no handle. The reason has already been
	/// reported through `Debug::Warn`, so the caller has nothing to add.
	Failed,
}
