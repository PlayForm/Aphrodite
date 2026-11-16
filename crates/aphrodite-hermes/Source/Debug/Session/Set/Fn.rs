//! Write (or clear) the debug flag for the current session tree.
//!
//! The tool entry point behind `aphrodite_debug`. It has to invalidate the mtime
//! cache by hand: a write within the same filesystem timestamp granularity would
//! otherwise leave the reader returning the previous value.

use anyhow::Context;

use crate::Debug::{
	Flag,
	Session::{Last, Root},
};

/// Set (or clear) the debug flag for the CURRENT session tree - the tool
/// entry point for `aphrodite_debug`. Returns the resolved root session id
/// and the flag path written, so the caller can report both.
///
/// Uses the persisted session id (falling back to `session.current` across
/// process restarts, same as `Last::Fn::Fn`): a fresh process wipes
/// `State::Current`, and without the file fallback the toggle would report "no
/// session context" until the next `pre_llm_call`.
///
/// Dev-only: the `aphrodite_debug` tool that calls this is gated behind
/// debug_assertions (release dylibs register exactly the 13 production
/// tools), so this entry point is dead code in release builds.
///
/// It does not create the runtime home - a missing home surfaces as the write
/// error, which is the honest answer to "the toggle did not stick".
pub(crate) fn Fn(on:bool) -> anyhow::Result<(String, String)> {
	let Tree = Root::Fn::Fn();
	let Tree = Tree.is_empty().then(Last::Fn::Fn).unwrap_or(Tree);

	anyhow::ensure!(
		!Tree.is_empty(),
		"no session context yet - hooks have not fired in this process"
	);

	let Path = Flag::Scoped::Fn::Fn(&Tree);
	let Text = on.then_some("on").unwrap_or("off");

	std::fs::write(&Path, Text).with_context(|| format!("write the debug flag {}", Path.display()))?;

	// Invalidate the mtime cache so the next read picks up the new value.
	let Key = Path.to_string_lossy().into_owned();

	// `Option::iter` is the zero-or-one loop: a cache that was never
	// initialised simply does not run the body.
	Flag::Cache::Map.get().iter().for_each(|Cache| {
		let mut Held = Cache.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
		Held.remove(&Key);
	});

	Ok((Tree, Path.to_string_lossy().into_owned()))
}
