//! Split content on UTF-8 char boundaries.

/// Split `Content` into chunks of at most `ChunkSize` BYTES, cutting only on
/// UTF-8 char boundaries so reassembly is lossless - a raw byte slice can split
/// a multi-byte char, and lossy re-encoding would corrupt the content and break
/// the round-trip contract.
///
/// The returned slices borrow from `Content`; nothing is copied here. A chunk may
/// be shorter than `ChunkSize` when a multi-byte character straddles the
/// boundary, which is the price of never splitting one.
///
/// The cut is a value, not a branch: `char_indices` yields the boundary
/// positions, and a position that has reached the next chunk's start closes the
/// current chunk. The tail is the same test one last time - content whose length
/// is an exact multiple ends with no remainder, and pushing an empty tail would
/// add a chunk the reassembly does not need.
pub(crate) fn Fn(Content:&str, ChunkSize:usize) -> Vec<&str> {
	let mut Chunks:Vec<&str> = Vec::new();
	let mut Start = 0usize;

	Content.char_indices().for_each(|(Index, _Character)| {
		(Index >= Start + ChunkSize).then(|| {
			Chunks.push(&Content[Start..Index]);
			Start = Index;
		});
	});

	Chunks.extend((Start < Content.len()).then(|| &Content[Start..]));

	Chunks
}
