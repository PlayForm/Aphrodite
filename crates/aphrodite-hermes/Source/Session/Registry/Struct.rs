//! The registry store: the process-global map, the fallback key, and the
//! get-or-create that seeds every session.
//!
//! This is the `Struct`-kind leaf of the `Registry` group - the storage the
//! rest of `Session/` reads through - and it deliberately owns nothing else.
//! The map's mutex is held for the get-or-create ONLY: never while hook work
//! runs, so an abandoned worker can never wedge the registry itself. Callers
//! receive an owned `Arc` clone so the registry lock is already released
//! before they contend on their session's state mutex.
//!
//! Each new session's state is a clone of [`config_template`], so a subagent
//! spawn inherits the parent's thresholds/directives/templates without
//! re-reading `aphrodite.toml`.

use std::{
	collections::HashMap,
	sync::{Arc, Mutex, OnceLock},
};

use aphrodite::State::AphroditeState;

use crate::Configuration::Template::Fn::config_template;

/// Key for calls with no session context (tools, tests, empty session_id).
pub(crate) const DEFAULT_SESSION:&str = "default";

/// Per-session state registry: session_id -> Arc<Mutex<AphroditeState>>.
///
/// Gets-or-creates: a new session's state is a clone of [`config_template`]
/// (config-seeded in production, plain defaults in test builds), so every
/// session starts from the same config without re-reading aphrodite.toml.
/// Returns an owned `Arc` clone so the registry lock is dropped before the
/// caller contends on the session's state mutex.
pub(crate) fn session_state(sid:&str) -> Arc<Mutex<AphroditeState>> {
	let key = if sid.is_empty() { DEFAULT_SESSION } else { sid };
	let mut map = SESSIONS
		.get_or_init(|| Mutex::new(HashMap::new()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner);
	map.entry(key.to_string())
		.or_insert_with(|| Arc::new(Mutex::new(config_template().clone())))
		.clone()
}

/// The registry itself. A plain static mutex with poison recovery: every
/// caller holds it for a HashMap lookup/insert and nothing more.
pub(crate) static SESSIONS:OnceLock<Mutex<HashMap<String, Arc<Mutex<AphroditeState>>>>> = OnceLock::new();
