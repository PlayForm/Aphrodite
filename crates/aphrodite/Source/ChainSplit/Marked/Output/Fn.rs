//! Split marked tool output back into per-segment pieces.

use crate::ChainSplit::{Const::SegMarker, Marked::Output::Part::Struct::Struct as Part};

/// Split tool output on segment markers into owned parts.
/// Output before the first marker belongs to segment 0. Marker lines are
/// removed; surrounding blank lines are trimmed per segment.
///
/// The parts are OWNED because the next stage consumes them: preparation moves
/// each part's text into its prepared segment, so the text is allocated once
/// here and never copied again. Borrowing slices of `Output` instead would tie
/// the batch to the caller's buffer and would have to re-derive the
/// normalization below, which is a change to segment text, not to allocation.
///
/// A line is one of two things - a boundary or text - so the split is driven by
/// that classification rather than by a boolean match with an empty arm. The
/// index advances on every boundary, whether or not the part it closed carried
/// text, because the index is the segment's chain position and not its rank
/// among the parts that survived.
pub fn Fn(Output:&str) -> Vec<Part> {
	let mut Parts:Vec<Part> = Vec::new();
	let mut Current = String::new();
	let mut CurrentIndex = 0usize;

	Output.lines().for_each(|Line| {
		if Line.trim() == SegMarker {
			let Text = Current.trim().to_string();

			let _ = (!Text.is_empty()).then(|| Parts.push(Part { Index:CurrentIndex, Text }));

			CurrentIndex += 1;
			// `clear()` rather than `Current = String::new()`: same observable
			// behaviour, but the buffer is reused - clippy's
			// `new_instead_of_clear` denies the reallocation under
			// `-D warnings` on the CI toolchain.
			Current.clear();
		} else {
			Current.push_str(Line);
			Current.push('\n');
		}
	});

	let Tail = Current.trim().to_string();
	let _ = (!Tail.is_empty()).then(|| Parts.push(Part { Index:CurrentIndex, Text:Tail }));

	Parts
}
