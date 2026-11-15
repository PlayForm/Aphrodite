//! One hash, one store lookup.
//!
//! This is the primitive the recursive resolver is built from: it answers "what is stored
//! under this hash" and deliberately does not look inside the answer. Unpacking nested
//! markers is `Recursive/`'s job, and keeping the two apart is what makes the depth limit
//! and the cycle guard expressible at all.
//!
//! It is not a validating parse: it tolerates a whole marker body because the model is not
//! reliable about stripping one, but it does not check that the hash is well formed, and it
//! never invents content for a hash the store does not hold.

use crate::State::Struct::AphroditeState;

/// Resolve a single CCR hash from the inline store.
///
/// Does NOT unpack nested markers. Returns `None` if the hash is not stored.
///
/// Tolerates a caller passing the full `hash|type|size` marker body instead of the bare
/// hash (an LLM copying a marker sometimes includes the pipe-delimited suffix instead of
/// stripping it) - the marker parser already splits on `|` for markers found in text, so
/// callers that hand us a raw argument (e.g. `aphrodite_retrieve(hash=...)`) need the same
/// tolerance.
///
/// A successful resolve of a chain-split segment hash is the tier-1 teaching loop's
/// consequence signal: it is attributed to the split event that produced the hash, counted
/// once per hash, so the threshold can adapt. Losing that attribution does not corrupt
/// content - it only blinds the loop, which is why it is an `inspect` on the found value
/// rather than a test beside it: the attribution runs exactly when there is a value to
/// attribute.
///
/// The `i:` prefix is the inline-only namespace, so it answers from the store and leaves:
/// there is no backend to fall back to for a hash that says it is local. Every other hash
/// is looked up in the same store, because the inline store is the read-through cache in
/// front of the CCR backend.
pub fn Fn(State:&mut AphroditeState, HashValue:&str) -> Option<String> {
	let HashValue = crate::Marker::Parse::Normalize::NormalizeHash(HashValue);

	let Some(Bare) = (!HashValue.starts_with("i:")).then_some(HashValue) else {
		return State.InlineStoreGet(HashValue);
	};

	// NOTE (F6): this used to check a `{hash}#stage2` key first, "for depth-aware
	// retrieval" - but nothing in the crate ever wrote that key, so it was pure dead-code
	// overhead (an extra store lookup on every resolve) with a latent trap: the moment any
	// future code *did* write `{hash}#stage2`, this check-before-standard-lookup ordering
	// would have made every plain resolution of `hash` silently and permanently return the
	// lossy reduced version instead of the original, with no way for a caller to opt out.
	// Wiring stage-2 up properly (a real `Depth` parameter on retrieve, only consulting
	// `#stage2` at `depth >= 2`) is a deliberate feature decision, not a bug fix - see
	// `.plans/05-compression-pipeline.md` §5. Deleted here rather than wired.
	let Found = State.InlineStoreGet(Bare);

	// Tier 1 teaching loop: a successful resolve of a chain-split segment hash is the
	// consequence signal - attribute it to the split event that produced it (counts once
	// per hash) and let the threshold adapt.
	let _ = Found.as_ref().map(|_Content| State.NoteSplitRetrieval(Bare));

	Found
}
