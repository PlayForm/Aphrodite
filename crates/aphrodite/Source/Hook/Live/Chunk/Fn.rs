//! Split content on UTF-8 char boundaries.

/// Split `Content` into chunks of at most `ChunkSize` BYTES, cutting only
/// on UTF-8 char boundaries so reassembly is lossless - a raw byte slice
/// can split a multi-byte char, and lossy re-encoding would corrupt the
/// content and break the round-trip contract.
///
/// The returned slices borrow from `Content`; nothing is copied here. A chunk
/// may be shorter than `ChunkSize` when a multi-byte character straddles the
/// boundary, which is the price of never splitting one.
pub(crate) fn Fn(Content:&str, ChunkSize:usize) -> Vec<&str> {
	let mut Chunks = Vec::new();
	let mut Start = 0usize;
	for (Index, _Character) in Content.char_indices() {
		if Index >= Start + ChunkSize {
			Chunks.push(&Content[Start..Index]);
			Start = Index;
		}
	}
	if Start < Content.len() {
		Chunks.push(&Content[Start..]);
	}
	Chunks
}
