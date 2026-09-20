use crate::state::AphroditeState;

/// Resolve a single CCR hash from the inline store.
/// Does NOT unpack nested markers. Returns None if not found.
///
/// Tolerates a caller passing the full `hash|type|size` marker body instead
/// of the bare hash (an LLM copying a marker sometimes includes the
/// pipe-delimited suffix instead of stripping it) - `parse_marker_hash`
/// already splits on `|` for markers found in text, so callers that hand us
/// a raw arg (e.g. `aphrodite_retrieve(hash=...)`) need the same tolerance.
pub fn resolve_one(state:&mut AphroditeState, hash_val:&str) -> Option<String> {
	let hash_val = crate::marker::normalize_hash(hash_val);

	// i: prefix - inline-only hashes
	if hash_val.starts_with("i:") {
		return state.inline_store_get(hash_val);
	}

	// NOTE (F6): this used to check a `{hash}#stage2` key first, "for
	// depth-aware retrieval" - but nothing in the crate ever wrote that key,
	// so it was pure dead-code overhead (an extra store lookup on every
	// resolve) with a latent trap: the moment any future code *did* write
	// `{hash}#stage2`, this check-before-standard-lookup ordering would have
	// made every plain resolution of `hash` silently and permanently return
	// the lossy reduced version instead of the original, with no way for a
	// caller to opt out. Wiring stage-2 up properly (a real `depth`
	// parameter on retrieve, only consulting `#stage2` at `depth >= 2`) is a
	// deliberate feature decision, not a bug fix - see
	// `.plans/05-compression-pipeline.md` §5. Deleted here rather than wired.
	let found = state.inline_store_get(hash_val);
	// Tier 1 teaching loop: a successful resolve of a chain-split segment
	// hash is the consequence signal - attribute it to the split event that
	// produced it (counts once per hash) and let the threshold adapt.
	if found.is_some() {
		state.note_split_retrieval(hash_val);
	}
	found
}
