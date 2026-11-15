//! One live directory watch: the event channel and the platform watcher behind
//! it.

use tokio::sync::mpsc::Receiver;

/// A running watch on one directory.
///
/// `Rx` yields one `()` per candidate event; the payload is deliberately
/// discarded at the source, because every event means the same thing ("the file
/// may have changed") and a burst is collapsed downstream.
///
/// `_Watcher` is held for one reason: a `notify` watcher that is dropped stops
/// delivering events. It carries no state anyone reads, so it is a private field
/// with a leading underscore rather than a named part of the value - a caller
/// that could reach it would have nothing to do with it.
pub(crate) struct Struct {
	/// One `()` per candidate event, in arrival order.
	pub(crate) Rx:Receiver<()>,
	_Watcher:notify::RecommendedWatcher,
}

impl Struct {
	/// Wrap a receiver together with the watcher that feeds it.
	pub(crate) fn new(Rx:Receiver<()>, Watcher:notify::RecommendedWatcher) -> Self { Self { Rx, _Watcher:Watcher } }
}
