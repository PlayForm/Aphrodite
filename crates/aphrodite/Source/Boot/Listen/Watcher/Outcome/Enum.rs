//! Whether a watch is live.

/// The result of asking for a config watch.
///
/// Two outcomes, because hot reload has two: it is running, or it is not and the
/// process will need a restart to see a new file. The earlier shape spawned a task
/// that returned `()` and created the platform watcher *inside* itself, so a watch
/// that could not be created produced a warning from a task nobody held and a caller
/// that could not tell "watching" from "not watching". Readiness is a fact the caller
/// has to be able to read.
pub enum Enum {
	/// The watch is live. The handle stops it.
	Watching(tokio::task::JoinHandle<()>),
	/// The watch could not be started. The process runs on the configuration it
	/// already loaded, and the reason was reported where the watch was created - a
	/// readiness fact does not have to carry a diagnostic the log already holds.
	Inactive,
}
