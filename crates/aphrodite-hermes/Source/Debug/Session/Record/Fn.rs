//! Learn a session's place in the tree and persist the current turn's id.
//!
//! `pre_llm_call` is the one hook Hermes threads both `session_id` and
//! `parent_session_id` through, so it is the only place the graph can be built.
//! Two side effects ride along: the root id is published to the core logger
//! (which writes `debug.<root>`), and the session id is mirrored to a file so a
//! fresh process can recover it.

use std::sync::Mutex;

use crate::Debug::{
	Runtime,
	Session::{Record::Resolve, State::LAST_SESSION},
};

/// Record session -> parent from a `pre_llm_call` invocation. Empty or
/// self-referential pairs are ignored (a root session has no parent).
///
/// It is not idempotent in its write side effect: every call rewrites
/// `session.current`, because the point of that file is to track the CURRENT
/// turn, not to be a one-time record. The parent edge, by contrast, is inserted
/// once (`or_insert_with`) so a later contradictory edge cannot rewrite history.
pub(crate) fn Fn(Session:&str, Parent:&str) {
	// `Option::iter` is the zero-or-one loop: a call with no session id has no
	// edge to record and no scope to publish, so nothing below runs for it.
	(!Session.is_empty())
		.then(|| {
			{
				let mut Last = LAST_SESSION
					.get_or_init(|| Mutex::new(String::new()))
					.lock()
					.unwrap_or_else(std::sync::PoisonError::into_inner);
				*Last = Session.to_string();
			}

			// Persist across process restarts: a fresh Hermes process (and every
			// separate shim exec) starts with empty Rust statics, so the session
			// id is mirrored to a tiny file that `Last::Fn::Fn` reads back.
			// Best-effort - a failed write degrades to "no persistence", never
			// an error.
			let _ = std::fs::write(Runtime::Fn::Fn().join("session.current"), Session);

			Resolve::Fn::Fn(Session, Parent)
		})
		.iter()
		.for_each(|Root| aphrodite::Debug::Root::Note::Fn::Fn(Root));
}
