//! The bridge's logging decision, handed to a sink it does not choose.

use aphrodite::State::Struct::AphroditeState;

use crate::Debug::Log::Level;

/// Decide whether to log `Event` for `Session`, and hand the record to `Sink`.
///
/// The sink is a parameter, not a process-global destination, and that is the
/// point of this leaf: WHAT the bridge decides (whether to log, at which level,
/// with which fields in which order) is separable from WHERE the record lands.
/// The decision can then be asserted without a subscriber, without a runtime
/// home, and - the reason the split exists - without mutating the process
/// environment to point the core sink at a scratch directory. A test that called
/// `unsafe { std::env::set_var }` to redirect the destination was unsafe for
/// reasons a test mutex cannot fix: `getenv` races `setenv` against every other
/// thread in the harness, not just against other tests.
///
/// `kv` is an ordered slice, not a map: the appended line keeps the caller's
/// field order, which is what makes two runs of the same event diffable.
pub(crate) fn Fn(
	State:&AphroditeState,
	Session:&str,
	Event:&str,
	Kv:&[(&str, String)],
	Sink:impl FnOnce(u8, &str, &[(&str, String)]),
) {
	let Effective = Level::Fn::Fn(State, Session);

	match Effective {
		// Quiet path: no event, no file append, no formatting.
		0 => {},
		Effective => Sink(Effective, Event, Kv),
	}
}
