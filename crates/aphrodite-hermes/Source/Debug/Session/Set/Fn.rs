//! Write (or clear) the debug flag for the current session tree.
//!
//! The tool entry point behind `aphrodite_debug`. It has to invalidate the mtime
//! cache by hand: a write within the same filesystem timestamp granularity would
//! otherwise leave the reader returning the previous value.

use anyhow::Context;

use crate::Debug::{
	Flag,
	Flag::Cache::FLAG_CACHE,
	Session::{Last, Root},
};

/// Set (or clear) the debug flag for the CURRENT session tree - the tool
/// entry point for `aphrodite_debug`. Returns the resolved root session id
/// and the flag path written, so the caller can report both.
///
/// Uses the persisted session id (falling back to `session.current` across
/// process restarts, same as `last_session`): a fresh process wipes
/// LAST_SESSION, and without the file fallback the toggle would report "no
/// session context" until the next `pre_llm_call`.
///
/// Dev-only: the `aphrodite_debug` tool that calls this is gated behind
/// debug_assertions (release dylibs register exactly the 13 production
/// tools), so this entry point is dead code in release builds.
///
/// It does not create the runtime home - a missing home surfaces as the write
/// error, which is the honest answer to "the toggle did not stick".
pub(crate) fn Fn(on:bool) -> anyhow::Result<(String, String)> {
	let root = Root::Fn::Fn();
	let root = root.is_empty().then(Last::Fn::Fn).unwrap_or(root);

	anyhow::ensure!(
		!root.is_empty(),
		"no session context yet - hooks have not fired in this process"
	);

	let flag = Flag::Scoped::Fn::Fn(&root);
	let flag_text = on.then_some("on").unwrap_or("off");

	std::fs::write(&flag, flag_text).with_context(|| format!("write the debug flag {}", flag.display()))?;

	// Invalidate the mtime cache so the next read picks up the new value.
	let key = flag.to_string_lossy().into_owned();

	// `Option::iter` is the zero-or-one loop: a cache that was never
	// initialised simply does not run the body.
	FLAG_CACHE.get().iter().for_each(|cache| {
		let mut guard = cache.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
		guard.remove(&key);
	});

	Ok((root, flag.to_string_lossy().into_owned()))
}
