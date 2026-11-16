//! The wake: three different reasons, three different values.
//!
//! Both properties used to be reachable only through a `bool` from the debounce,
//! which could say "an event happened" or "the channel closed" and nothing else.
//! A stop request had no representation at all, so the loop could only learn
//! about it by polling a flag between events - and on a quiet config file there
//! is no next event.

use std::sync::{atomic::AtomicBool, mpsc::channel};

use crate::Session::Watcher::{Event, Wait};

// ── A closed channel ends the watch: `recv` fails and the loop stops instead of
// blocking the watcher thread forever. ──
#[test]
fn probe_wait_reports_a_closed_channel() {
	let (Sender, Receiver) = channel::<()>();
	drop(Sender);

	let Stop = AtomicBool::new(false);

	assert!(
		matches!(Wait::Fn::Fn(&Receiver, &Stop), Event::Enum::Enum::Closed),
		"a closed channel must end the watch, not block on it"
	);
}

// ── A stop request ends the watch even though a wake arrived: the flag is
// checked before the wake is accepted as a tick, so a stop that races a file
// event still stops. ──
#[test]
fn probe_wait_reports_a_stop_request() {
	let (Sender, Receiver) = channel::<()>();
	let Stop = AtomicBool::new(true);
	Sender.send(()).unwrap();

	assert!(
		matches!(Wait::Fn::Fn(&Receiver, &Stop), Event::Enum::Enum::Stop),
		"a stop request must end the watch, not be consumed as a config tick"
	);
}

// ── A pending event with no stop request is exactly one tick, and the burst is
// drained by it (the drain is `Debounce`'s, proved there). ──
#[test]
fn probe_wait_reports_a_tick() {
	let (Sender, Receiver) = channel::<()>();
	let Stop = AtomicBool::new(false);
	Sender.send(()).unwrap();

	assert!(
		matches!(Wait::Fn::Fn(&Receiver, &Stop), Event::Enum::Enum::Tick),
		"a pending event with no stop request is one tick"
	);
}
