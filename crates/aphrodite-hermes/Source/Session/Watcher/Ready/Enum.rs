//! The watcher thread's readiness report, sent once, before it loops.

/// What the watcher thread learned while coming up.
///
/// Readiness is a message, not an assumption: `std::thread::Builder::spawn`
/// succeeding proves a thread exists, not that a notifier was built or that any
/// directory is being watched. Without this report the spawner had no way to
/// tell "the watcher is running" from "the watcher thread returned immediately
/// because `notify` refused to start", and `auto_reload = true` silently did
/// nothing.
pub(crate) enum Enum {
	/// The notifier exists and every directory that could be watched is being
	/// watched. (A directory that could not be watched is a warning the thread
	/// already reported; it does not fail readiness, because the runtime home
	/// may simply not exist yet.)
	Watching,
	/// The notifier could not be built at all. The reason is the diagnostic the
	/// spawner reports; there is nothing else the thread can do.
	Failed(String),
}
