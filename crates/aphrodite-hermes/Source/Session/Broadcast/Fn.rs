//! The apply-to-every-session sweep, used by the config auto-reload watcher.

use std::sync::{Arc, Mutex};

use aphrodite::State::AphroditeState;

use crate::Session::{Lock::Fn::lock_bounded, Registry::Struct::SESSIONS};

/// Apply `f` to EVERY registered session under a BOUNDED lock, skipping
/// wedged sessions (an abandoned worker holds their mutex - a config reload
/// must never block the watcher; the reload reaches them on a later pass).
///
/// Returns true when any session still has `auto_reload` on after the pass
/// (the watcher keeps running only while at least one session wants it).
pub(crate) fn for_each_session(f:impl Fn(&mut AphroditeState)) -> bool {
	let map = SESSIONS
		.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner);
	// Clone the Arcs so the registry lock is released before contending on
	// any session state mutex.
	let sessions:Vec<Arc<Mutex<AphroditeState>>> = map.values().cloned().collect();
	drop(map);
	let mut any_reload = false;
	for state in sessions {
		if let Some(mut guard) = lock_bounded(&state) {
			f(&mut guard);
			if guard.AutoReload {
				any_reload = true;
			}
		}
		// Wedged session: skip (fail-open - it stays degraded, never blocks).
	}
	any_reload
}
