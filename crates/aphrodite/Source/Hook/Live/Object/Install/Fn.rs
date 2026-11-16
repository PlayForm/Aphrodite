//! Install a live container's views into the inline store.

use crate::{
	Hook::Live::{
		Chunk::Policy::Struct::Struct as ChunkPolicy,
		Const::LiveChunkSize,
		Key::Fn::{LiveChunkKey, LiveMetaKey},
		Object::Struct::Struct as Object,
	},
	State::Struct::AphroditeState,
};

/// Store every view of a live container, and answer the object that was stored.
///
/// Three views of one object, written in one place because they are one object:
/// the whole content (the lossless round-trip entry `Resolve::Expand` serves),
/// one entry per chunk (what the retrieve path serves progressively), and the
/// metadata envelope (what tells the retrieve path the first two exist). Both
/// representations are intentional - full expansion and progressive retrieval
/// have different consumers - so neither is redundant.
///
/// The chunk count is counted as the chunks are stored, not predicted from the
/// policy: the envelope is written last and describes what is actually in the
/// store. A policy that cannot be built (`LiveChunkSize` is zero) stores the
/// whole content and no chunks, which the retrieve path already reads as "not a
/// container" and serves as a plain entry - a degradation, never a lie.
pub(crate) fn Fn(State:&mut AphroditeState, Content:&str, TypeStr:&str) -> Object {
	let Hash = headroom_core::ccr::compute_key(Content.as_bytes());
	let Total = Content.len();
	let Policy = ChunkPolicy::new(LiveChunkSize);

	// Round-trip view: the plain hash resolves to the exact original.
	State.InlineStorePut(Hash.clone(), Content.to_string());

	// Chunk views: produced lazily, stored and counted in the same pass.
	let Chunks = Policy.as_ref().map_or(0, |Policy| {
		let mut Count = 0usize;

		for Chunk in Policy.Chunks(Content) {
			State.InlineStorePut(LiveChunkKey(&Hash, Count), Chunk.to_string());
			Count += 1;
		}

		Count
	});
	// The envelope reports the target the chunks were actually cut with, and zero
	// when there was no policy to cut with - a container with no chunks, which the
	// retrieve path reads as "not a container".
	let ChunkSize = Policy.as_ref().map_or(0, |Policy| Policy.Target());

	let Object = Object { Hash, Type:TypeStr.to_string(), Total, Chunks, ChunkSize };
	State.InlineStorePut(LiveMetaKey(&Object.Hash), Object.Meta());

	Object
}
