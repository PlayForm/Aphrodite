//! One turn of the watcher loop, as three different wakes.

/// Why the watcher loop woke.
///
/// The loop used to learn this from a `bool` returned by the debounce: `true`
/// meant "an event happened", `false` meant "the channel closed". That left no
/// way to say the third thing the loop must handle - the handle asked it to
/// stop - and it made "the channel closed" (an expected termination: the
/// notifier was dropped) indistinguishable from an event burst.
pub(crate) enum Enum {
	/// One config-change tick: the burst that arrived while the editor's write
	/// settled has been drained, so this is the last write of the burst.
	Tick,
	/// The handle asked the loop to stop. Expected termination, not a failure.
	Stop,
	/// The channel closed: the notifier that feeds it is gone. Expected
	/// termination too, and the reason the loop must not treat a closed channel
	/// as a tick it should keep retrying.
	Closed,
}
