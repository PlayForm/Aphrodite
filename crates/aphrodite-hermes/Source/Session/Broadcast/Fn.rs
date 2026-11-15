//! The apply-to-every-session sweep, used by the config auto-reload watcher.

use std::sync::{Arc, Mutex};

use aphrodite::State::Struct::AphroditeState;

use crate::Session::{Lock::Fn::lock_bounded, Registry::Struct::SESSIONS};

/// Apply `f` to EVERY registered session under a BOUNDED lock, skipping
/// wedged sessions (an abandoned worker holds their mutex - a config reload
/// must never block the watcher; the reload reaches them on a later pass).
///
/// Returns true when any session still has `auto_reload` on after the pass
/// (the watcher keeps running only while at least one session wants it).
///
/// The Arcs are cloned inside the registry guard's SCOPE so the registry lock is
/// released before contending on any session state mutex. A scope ends it, not a
/// `drop` call: `MutexGuard` is a `Drop` type that lives to the end of its scope,
/// so the scope is the thing that has to be right.
pub(crate) fn Fn(f:impl Fn(&mut AphroditeState)) -> bool {
	let Sessions:Vec<Arc<Mutex<AphroditeState>>> = {
		let Map = SESSIONS
			.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner);

		Map.values().cloned().collect()
	};

	Sessions.into_iter().fold(false, |AnyReload, State| {
		match lock_bounded(&State) {
			Some(mut Guard) => {
				f(&mut Guard);

				AnyReload || Guard.AutoReload
			},
			// Wedged session: skip (fail-open - it stays degraded, never blocks).
			None => AnyReload,
		}
	})
}
