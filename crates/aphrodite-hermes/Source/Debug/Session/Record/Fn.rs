//! Learn a session's place in the tree and persist the current turn's id.
//!
//! `pre_llm_call` is the one hook Hermes threads both `session_id` and
//! `parent_session_id` through, so it is the only place the graph can be built.
//! Two side effects ride along: the root id is published to the core logger
//! (which writes `debug.<root>`), and the session id is mirrored to a file so a
//! fresh process can recover it.

use std::{collections::HashMap, sync::Mutex};

use Debug::Chain::self;
use Debug::Chain::State::SESSION_PARENTS;
use Debug::Runtime;
use Debug::Session::State::LAST_SESSION;

/// Record session -> parent from a `pre_llm_call` invocation. Empty or
/// self-referential pairs are ignored (a root session has no parent).
///
/// It is not idempotent in its write side effect: every call rewrites
/// `session.current`, because the point of that file is to track the CURRENT
/// turn, not to be a one-time record. The parent edge, by contrast, is inserted
/// once (`or_insert_with`) so a later contradictory edge cannot rewrite history.
pub(crate) fn Fn(session:&str, parent:&str) {
	if session.is_empty() {
		return;
	}
	{
		let mut last = LAST_SESSION
			.get_or_init(|| Mutex::new(String::new()))
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner);
		*last = session.to_string();
	}
	// Persist across process restarts: a fresh Hermes process (and every
	// separate shim exec) starts with empty Rust statics, so the session id
	// is mirrored to a tiny file that `last_session()` reads back.
	// Best-effort - a failed
	// write degrades to "no persistence", never an error.
	let _ = std::fs::write(Runtime::Fn::Fn().join("session.current"), session);
	if parent.is_empty() || session == parent {
		// Root session (no parent): publish it directly so the env-gated
		// logger targets `debug.<root>` (crate::debug).
		aphrodite::Debug::Root::Note::Fn::Fn(session);
		return;
	}
	let mut map = SESSION_PARENTS
		.get_or_init(|| Mutex::new(HashMap::new()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner);
	map.entry(session.to_string()).or_insert_with(|| parent.to_string());
	// Publish the ROOT of this session tree (root-most ancestor) so the
	// env-gated logger targets `debug.<root>` for the whole tree.
	let root = Chain::Fn(session).last().cloned().unwrap_or_else(|| session.to_string());
	aphrodite::Debug::Root::Note::Fn::Fn(&root);
}
