use std::collections::HashMap;

use crate::state::AphroditeState;
use super::{one::resolve_one, parse::find_markers};

// Depth limit is `AphroditeState::recursive_depth` - config-driven via
// `APHRODITE_RECURSIVE_DEPTH`, default 5 in `AphroditeState::default()` and
// the config loader, so the pre-port hardcoded constant's value is
// byte-identical when unset. Every entry point in this module takes the
// state, so there is no state-less path needing its own fallback.

/// Recursively resolve a hash and all nested CCR markers.
///
/// Cycle-safe via `visited` set. Uses `resolved` map as a persistent
/// cache across the entire resolution tree - once a hash is resolved,
/// nested references to it reuse the cached result.
///
/// Returns `Some(resolved_content)` on success, or `None` if the
/// top-level hash could not be found.
pub fn resolve_recursive(
	state:&mut AphroditeState,
	hash_val:&str,
	depth:usize,
	resolved:&mut HashMap<String, String>,
	visited:&mut Vec<String>,
) -> Option<String> {
	// Cycle detection
	if visited.contains(&hash_val.to_string()) {
		return resolved.get(hash_val).cloned();
	}
	visited.push(hash_val.to_string());

	// Depth limit (config-driven via `state.recursive_depth`,
	// `APHRODITE_RECURSIVE_DEPTH`, default 5 - byte-identical to the
	// pre-port hardcoded constant when unset): return the raw
	// (un-further-expanded) content for this hash rather than falling
	// back to the resolved cache (F9) - a hash that legitimately exists
	// but simply hasn't been visited yet at this depth would otherwise
	// incorrectly resolve to `None`.
	if depth >= state.recursive_depth {
		return resolve_one(state, hash_val);
	}
	if let Some(cached) = resolved.get(hash_val) {
		return Some(cached.clone());
	}

	// Resolve the top-level hash
	let content = resolve_one(state, hash_val)?;
	resolved.insert(hash_val.to_string(), content.clone());

	// Find nested CCR markers
	let nested_markers = find_markers(&content);
	if nested_markers.is_empty() {
		return Some(content);
	}

	// Resolve nested hashes and build replacements. A nested hash already
	// in `resolved` still needs a replacement pushed using the CACHED value
	// (F4) - previously it was silently skipped, leaving the literal marker
	// in the output (which F1's write-back would then have persisted).
	let mut replacements:Vec<(String, Option<String>)> = Vec::new();
	for (marker, nested_hash) in &nested_markers {
		let nested_content = if let Some(cached) = resolved.get(nested_hash) {
			Some(cached.clone())
		} else {
			resolve_recursive(state, nested_hash, depth + 1, resolved, visited)
		};
		replacements.push((marker.clone(), nested_content));
	}

	// Apply replacements. An unresolved nested hash leaves the ORIGINAL
	// marker text untouched (F1) - substituting `[CCR_UNRESOLVED:...]` here
	// used to get permanently baked into the store below, so a merely
	// evicted/late nested entry could never heal even after it reappeared.
	let mut result = content;
	for (marker, replacement) in &replacements {
		if let Some(repl) = replacement {
			result = result.replace(marker.as_str(), repl.as_str());
		}
	}

	// NOTE (F1): the expanded result is intentionally NOT written back over
	// `hash_val` in the store. Doing so previously broke the content-address
	// invariant (the stored bytes no longer matched the hash that names
	// them) and destroyed the pristine original irrecoverably - including
	// any literal `<<<CCR:...>>>`-shaped text the original content merely
	// *contained* (this crate's own test fixtures, docs, echoed tool
	// output), which got silently corrupted on every retrieval.
	Some(result)
}

/// Auto-expand variant of [`resolve_recursive`]: resolves `hash_val` and
/// all nested markers with the same cycle-safe `visited` set and
/// `state.recursive_depth` limit, but returns the resolved content ONLY
/// when it fits within `limit` bytes - the "resolve inline instead of
/// emitting a marker" contract for auto-expand (pre-Rust `_resolve.py` +
/// `_core/config.py` AUTO_EXPAND_LIMIT).
///
/// No marker is re-emitted: when the content fits, the returned string IS
/// the fully-resolved content. Over-limit results return `None` so the
/// caller keeps the original marker (the stored bytes remain retrievable
/// for a later, explicit resolve). A `None` is also returned when the
/// top-level hash cannot be resolved at all, indistinguishable from an
/// over-limit result by design - the caller's marker fallback is correct
/// for both.
pub fn resolve_inline(
	state:&mut AphroditeState,
	hash_val:&str,
	limit:usize,
	resolved:&mut HashMap<String, String>,
	visited:&mut Vec<String>,
) -> Option<String> {
	let content = resolve_recursive(state, hash_val, 0, resolved, visited)?;
	if content.len() <= limit { Some(content) } else { None }
}
