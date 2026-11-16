//! The watcher's lifecycle: a stop flag, a wake channel, and a joined thread.

use std::{
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
		mpsc::Sender,
	},
	thread::JoinHandle,
};

/// Everything needed to end the config watcher and to wait for it to be gone.
///
/// A detached thread cannot be stopped, cannot be waited for, and - in this
/// dylib - must not outlive its own code: Hermes hot-reloads the dylib on mtime
/// change, and a thread still executing inside an unmapped image is undefined
/// behaviour. So the thread is joined rather than detached, and the handle is
/// the thing that makes joining possible.
///
/// `stop` is called on the one path that can replace a running watcher (see
/// `Watcher::Fn::spawn_config_watcher`); the process-lifetime watcher ends with
/// the process, which is why nothing else calls it. The handle exists so that
/// "the process exits" is not the ONLY way the thread can end.
pub(crate) struct Struct {
	/// Set before the wake is sent. The loop checks it as the first thing after
	/// waking, so a stop that races an event still ends the watch.
	pub Stop:Arc<AtomicBool>,
	/// A sender into the SAME channel the notifier feeds. The stop flag alone
	/// would leave the loop blocked in `recv` until the next file event, which
	/// on a quiet config file never comes.
	pub Wake:Sender<()>,
	/// The thread itself. Joined in `stop`, never detached.
	pub Joined:JoinHandle<()>,
}

impl Struct {
	/// Ask the watcher to stop, then wait for the thread to return.
	///
	/// The order is load-bearing: flag first (so the wake is not consumed as an
	/// ordinary tick), wake second (so the loop leaves `recv` now rather than at
	/// the next file event), join last (so the caller knows no thread of this
	/// image is still running when it returns).
	pub(crate) fn stop(self) {
		self.Stop.store(true, Ordering::Release);
		let _ = self.Wake.send(());
		let _ = self.Joined.join();
	}
}
