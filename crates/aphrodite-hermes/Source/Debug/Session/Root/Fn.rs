//! The root of the most recently seen session's tree.
//!
//! The debug toggle is root-scoped, and the tool that flips it carries no
//! session id - so the root has to be derived from the last session seen. This
//! is the derivation, kept separate from the write so the tool can report the
//! resolved root without touching a file.

use std::sync::Mutex;

use crate::Debug::{Chain, Session::State::LAST_SESSION};

/// Resolve the most recently seen session's ROOT id.
///
/// Returns an empty string when no session has been recorded yet, which the
/// toggle reports as "no session context" rather than inventing a root.
///
/// The `LAST_SESSION` guard is scoped to the clone, not held across the walk:
/// the walk takes `SESSION_PARENTS`, and holding one process-global guard while
/// acquiring another is how a lock-order inversion is born.
///
/// Dev-only: the only caller is the `aphrodite_debug` tool, gated behind
/// `debug_assertions` so release dylibs register exactly the 13 production
/// tools.
pub(crate) fn Fn() -> String {
	let Last = {
		let Guard = LAST_SESSION
			.get_or_init(|| Mutex::new(String::new()))
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner);

		Guard.clone()
	};

	Last.is_empty()
		.then(String::new)
		.unwrap_or_else(|| Chain::Fn::Fn(&Last).last().cloned().unwrap_or_else(|| Last.clone()))
}
