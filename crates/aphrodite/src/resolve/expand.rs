use std::collections::HashMap;

use crate::state::AphroditeState;
use super::recursive::resolve_recursive;

/// Public entry point: expands a hash through all nesting levels.
pub fn expand(state:&mut AphroditeState, hash_val:&str) -> Option<String> {
	// DEV-ONLY debug instrumentation (crate::debug): retrieval metrics +
	// one structured line per retrieval request (hash, found, depth cap,
	// elapsed). Captured only when the logger is on.
	let t0 = crate::debug::enabled().then(std::time::Instant::now);
	crate::debug::bump(crate::debug::Counter::Retrieves);
	let mut resolved = HashMap::new();
	let mut visited = Vec::new();
	let out = resolve_recursive(state, hash_val, 0, &mut resolved, &mut visited);
	if let Some(t0) = t0 {
		let us = t0.elapsed().as_micros() as u64;
		crate::debug::add_timing(crate::debug::TimingCat::Retrieve, us);
		crate::debug::log_at(
			state.debug_level,
			"retrieve",
			&[
				("hash", hash_val.to_string()),
				("found", out.is_some().to_string()),
				("depth_cap", state.recursive_depth.to_string()),
				("elapsed_us", us.to_string()),
			],
		);
		if out.is_none() {
			// A miss degrades to a warning (dev metrics).
			crate::debug::bump(crate::debug::Counter::RetrieveMisses);
			crate::debug::bump(crate::debug::Counter::Warnings);
		}
	}
	out
}
