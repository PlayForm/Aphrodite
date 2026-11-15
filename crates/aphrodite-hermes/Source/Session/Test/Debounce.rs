//! The debounce controller: a burst of events is one tick, a closed channel is
//! the end of the watch.
//!
//! Both properties used to be reachable only by touching the repo's live
//! `aphrodite.toml`, because the debounce lived inside the watcher thread's
//! closure. It is a function of a channel now, so the proof is a channel.

use std::sync::mpsc::channel;

use crate::Session::Watcher::Debounce::Fn::Fn as debounce;

// ── A closed channel ends the watch: `recv` fails and the caller's loop
// stops instead of blocking the watcher thread forever. ──
#[test]
fn probe_debounce_reports_a_closed_channel() {
	let (Sender, Receiver) = channel::<()>();
	drop(Sender);

	assert!(!debounce(&Receiver), "a closed channel must end the watch, not block on it");
}

// ── A burst is ONE tick, and the events that arrived during the settle window
// are drained: without the drain, five saves within 200ms would reload five
// times and the last write could be applied before the editor finished it. ──
#[test]
fn probe_debounce_collapses_a_burst_into_one_tick() {
	let (Sender, Receiver) = channel::<()>();
	(0..5).for_each(|_| Sender.send(()).unwrap());

	assert!(debounce(&Receiver), "a pending event must yield a tick");
	assert!(
		Receiver.try_recv().is_err(),
		"the burst must be drained by the tick that consumed it"
	);
}
