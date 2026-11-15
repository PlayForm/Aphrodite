//! Collapse a burst of file events into one tick.

use std::sync::mpsc::Receiver;

/// Wait for the next config-change tick, then drain the burst that accumulated
/// while the editor's write settled.
///
/// The settle window is the same 500ms the engine watcher uses, and it is what
/// makes one save (which fires several events) one reload rather than five.
///
/// Returns false when the channel closed, which is how the watcher thread learns
/// it is being dropped. It knows nothing about configuration or sessions, so the
/// debounce can be exercised with a channel alone.
pub(crate) fn Fn(Receiver:&Receiver<()>) -> bool {
	Receiver
		.recv()
		.map(|()| {
			std::thread::sleep(std::time::Duration::from_millis(500));
			Receiver.try_iter().for_each(drop);
		})
		.is_ok()
}
