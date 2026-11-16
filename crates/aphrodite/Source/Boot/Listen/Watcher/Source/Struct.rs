//! One live directory watch: the event channel and the platform watcher behind it.

use tokio::sync::mpsc::Receiver;

use crate::Boot::Listen::Watcher::Target;

/// A running watch on one directory.
///
/// `Rx` yields the resolved target per candidate event. It used to yield `()`, which
/// threw away the one fact the applier needs - which file changed - and forced the
/// path to be threaded alongside the channel as a second, parallel argument that
/// nothing kept in step with the events. The payload is the target, so the applier
/// reads what changed from the event itself.
///
/// `_Watcher` is held for one reason: a `notify` watcher that is dropped stops
/// delivering events. It carries no state anyone reads, so it is a private field with
/// a leading underscore rather than a named part of the value - a caller that could
/// reach it would have nothing to do with it.
pub(crate) struct Struct {
	/// One resolved target per candidate event, in arrival order.
	pub(crate) Rx:Receiver<Target::Struct::Struct>,
	_Watcher:notify::RecommendedWatcher,
}

impl Struct {
	/// Wrap a receiver together with the watcher that feeds it.
	pub(crate) fn new(Rx:Receiver<Target::Struct::Struct>, Watcher:notify::RecommendedWatcher) -> Self {
		Self { Rx, _Watcher:Watcher }
	}
}
