//! Lazy production of chunks on UTF-8 char boundaries.

use std::{iter::FusedIterator, num::NonZeroUsize};

/// A borrowed chunk iterator: the remaining content and the target size.
///
/// Lazy because the container stores each chunk as it is produced: collecting a
/// `Vec<&str>` first would allocate a descriptor per chunk that is thrown away in
/// the same pass. The items are borrowed slices of the content - the chunk
/// payload allocation is the store's, not the iterator's.
///
/// Progress is guaranteed by the type: `Target` is non-zero, so every step cuts
/// at least one byte, and the remaining slice strictly shrinks until it is empty.
pub(crate) struct Struct<'a> {
	Remaining:&'a str,
	Target:NonZeroUsize,
}

impl<'a> Struct<'a> {
	pub(crate) fn new(Content:&'a str, Target:NonZeroUsize) -> Struct<'a> { Struct { Remaining:Content, Target } }
}

impl<'a> Iterator for Struct<'a> {
	type Item = &'a str;

	fn next(&mut self) -> Option<&'a str> {
		// Exhaustion is the empty remaining slice itself, not a guard in front of
		// it: the `?` is the `None` this iterator produces.
		let Remaining = (!self.Remaining.is_empty()).then_some(self.Remaining)?;

		// The cut is the first char boundary at or beyond the target, or the end
		// of the content when no boundary is that far out. `char_indices` never
		// yields a mid-character index, which is what makes the split lossless.
		let Cut = Remaining
			.char_indices()
			.find(|(Index, _)| *Index >= self.Target.get())
			.map(|(Index, _)| Index)
			.unwrap_or(Remaining.len());

		let (Chunk, Rest) = Remaining.split_at(Cut);
		self.Remaining = Rest;

		Some(Chunk)
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		// At least one chunk remains while anything does; at most one chunk per
		// byte is the only bound the target guarantees without measuring.
		match self.Remaining.is_empty() {
			true => (0, Some(0)),
			false => (1, Some(self.Remaining.len())),
		}
	}
}

/// Exhaustion is permanent: `next` only ever shrinks `Remaining`, and an empty
/// remaining slice is the exhaustion condition itself.
impl<'a> FusedIterator for Struct<'a> {}
