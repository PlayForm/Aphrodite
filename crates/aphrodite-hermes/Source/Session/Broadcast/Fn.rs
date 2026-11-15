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
pub(crate) fn for_each_session(f:impl Fn(&mut AphroditeState)) -> bool {
	let map = SESSIONS
		.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner);
	// Clone the Arcs so the registry lock is released before contending on
	// any session state mutex.
	let sessions:Vec<Arc<Mutex<AphroditeState>>> = map.values().cloned().collect();
	drop(map);

	sessions.into_iter().fold(false, |any_reload, state| {
		match lock_bounded(&state) {
			Some(mut guard) => {
				f(&mut guard);

				any_reload || guard.AutoReload
			},
			// Wedged session: skip (fail-open - it stays degraded, never blocks).
			None => any_reload,
		}
	})
}
