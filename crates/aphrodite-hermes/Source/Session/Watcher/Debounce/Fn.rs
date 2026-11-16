//! Collapse a burst of file events into one tick.

use std::sync::mpsc::Receiver;

/// Wait for the next config-change tick, then drain the burst that accumulated
/// while the editor's write settled.
///
/// The settle window is the same 500ms the engine watcher uses, and it is what
/// makes one save (which fires several events) one reload rather than five.
///
/// Returns false when the channel closed, which is how the loop learns that the
/// notifier feeding it is gone. It knows nothing about configuration or sessions,
/// so the debounce can be exercised with a channel alone - and it deliberately
/// knows nothing about the stop flag either: `Wait::Fn::Fn` is what wraps this and
/// names the three ways the loop can wake, so this leaf stays the settle-and-drain
/// and nothing else.
pub(crate) fn Fn(Receiver:&Receiver<()>) -> bool {
	Receiver
		.recv()
		.map(|()| {
			std::thread::sleep(std::time::Duration::from_millis(500));
			Receiver.try_iter().for_each(drop);
		})
		.is_ok()
}
