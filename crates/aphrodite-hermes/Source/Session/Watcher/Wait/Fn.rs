//! Wait for the next wake and say which kind it was.

use std::sync::{
	atomic::{AtomicBool, Ordering},
	mpsc::Receiver,
};

use crate::Session::Watcher::{Debounce, Event};

/// Block until the notifier fires, the handle stops the loop, or the channel
/// closes - and report which of the three happened.
///
/// The stop flag is checked AFTER `Debounce` returns, and that ordering is the
/// point: `Debounce` is what owns the `recv` (a blocking wait with no timeout,
/// which is why an idle watcher costs nothing) plus the 500ms settle and the
/// drain. Wrapping it means the stop path reuses the same wake - the handle
/// sends into the event channel - instead of needing a second channel, a
/// `recv_timeout` poll loop, or a way to interrupt a blocking receive that
/// `std::sync::mpsc` does not have.
///
/// The cost is that a stop waits out the current settle window (at most 500ms)
/// rather than interrupting it. That is the right trade: a config watcher is
/// stopped when the process is going away, and losing a drained burst is
/// harmless, while a second wake channel would be a second thing to get wrong.
pub(crate) fn Fn(Receiver:&Receiver<()>, Stop:&AtomicBool) -> Event::Enum::Enum {
	match Debounce::Fn::Fn(Receiver) {
		false => Event::Enum::Enum::Closed,
		true => {
			match Stop.load(Ordering::Acquire) {
				true => Event::Enum::Enum::Stop,
				false => Event::Enum::Enum::Tick,
			}
		},
	}
}
