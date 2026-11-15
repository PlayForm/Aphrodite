//! Resolution entry point: a hash in, the content it names out.
//!
//! Every retrieval path in the crate funnels here - the ABI retrieve export, the
//! proxy relay, the hooks, the Hermes retrieve tool - so this function owns the
//! whole nesting contract: resolve the hash, resolve the markers inside it,
//! resolve theirs, up to `AphroditeState::recursive_depth`. It is not a store
//! lookup; `One/` is the lookup, and this wraps it.
//!
//! It is also the retrieval metric's only sampling point: the timing, the
//! found/miss counter and the one structured line per request are recorded here,
//! so a caller that bypasses this function is invisible to the metrics - which is
//! why the ABI retrieve path, which resolves directly, has to attribute its own
//! consequence signals.

use std::collections::HashMap;

use crate::State::Struct::AphroditeState;
use super::Recursive::Recursive;

/// Expand a hash through every nesting level.
///
/// Returns the fully expanded content, or `None` when the top-level hash is not
/// in the store. A nested hash that cannot be resolved is left as its original
/// marker text rather than replaced with a placeholder - that entry may only be
/// evicted or late, and a placeholder baked into the output can never heal.
///
/// It is not a compressor and it never writes back: the stored bytes under a
/// hash must stay byte-identical to what the hash names, so an expanded result is
/// returned and forgotten. Writing it back breaks the content-address invariant
/// and destroys the pristine original irrecoverably - including any marker-shaped
/// text the original merely contained.
pub fn Fn(State:&mut AphroditeState, HashValue:&str) -> Option<String> {
	// DEV-ONLY debug instrumentation (crate::debug): retrieval metrics +
	// one structured line per retrieval request (hash, found, depth cap,
	// elapsed). Captured only when the logger is on.
	let T0 = crate::Debug::Level::Enabled::Fn::Fn().then(std::time::Instant::now);
	crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::Retrieves);
	let mut Resolved = HashMap::new();
	let mut Visited = Vec::new();
	let Out = Recursive::Fn(State, HashValue, 0, &mut Resolved, &mut Visited);
	match T0 {
		Some(T0) => {
			let Us = T0.elapsed().as_micros() as u64;
			crate::Debug::Timing::Add::Fn::Fn(crate::Debug::Timing::Enum::Enum::Retrieve, Us);
			crate::Debug::Log::At::Fn::Fn(
				State.DebugLevel,
				"retrieve",
				&[
					("hash", HashValue.to_string()),
					("found", Out.is_some().to_string()),
					("depth_cap", State.RecursiveDepth.to_string()),
					("elapsed_us", Us.to_string()),
				],
			);

			match Out.is_none() {
				true => {
					// A miss degrades to a warning (dev metrics).
					crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::RetrieveMisses);
					crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::Warnings);
				},
				false => (),
			}
		},
		None => (),
	}

	Out
}
