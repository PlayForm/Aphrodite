//! The apply-to-every-session sweep, used by the config auto-reload watcher.

use std::sync::{Arc, Mutex};

use aphrodite::State::Struct::AphroditeState;

use crate::Session::{
	Broadcast::Report,
	Lock::{Fn::lock_bounded, Outcome},
	Registry::Struct::SESSIONS,
};

/// Apply `f` to EVERY registered session under a BOUNDED lock, skipping
/// wedged sessions (an abandoned worker holds their mutex - a config reload
/// must never block the watcher; the reload reaches them on a later pass).
///
/// Answers with a [`Report::Struct::Struct`] of counts, not a `bool`: the caller can
/// then tell "every session turned auto-reload off" from "every session's lock
/// was unavailable" and from "no session was registered yet". See
/// `Report::Struct::Struct` for why that distinction is load-bearing.
///
/// The Arcs are cloned inside the registry guard's SCOPE so the registry lock is
/// released before contending on any session state mutex. A scope ends it, not a
/// `drop` call: `MutexGuard` is a `Drop` type that lives to the end of its scope,
/// so the scope is the thing that has to be right.
pub(crate) fn Fn(f:impl Fn(&mut AphroditeState)) -> Report::Struct::Struct {
	let Sessions:Vec<Arc<Mutex<AphroditeState>>> = {
		let Map = SESSIONS
			.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner);

		Map.values().cloned().collect()
	};

	Sessions.into_iter().fold(
		Report::Struct::Struct { Applied:0, Skipped:0, WantsReload:0 },
		|mut Report, State| {
			match lock_bounded(&State) {
				Outcome::Enum::Enum::Held(mut Guard) => {
					f(&mut Guard);

					Report.Applied += 1;
					Report.WantsReload += usize::from(Guard.AutoReload);
				},
				// Wedged session: count it and move on (fail-open - it stays
				// degraded, and the caller decides whether that is a reason to
				// keep watching).
				Outcome::Enum::Enum::Wedged => Report.Skipped += 1,
			}

			Report
		},
	)
}
