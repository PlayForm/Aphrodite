//! Split marked tool output back into per-segment pieces.

use crate::ChainSplit::Const::SegMarker;

/// Split tool output on segment markers into (segment_index, text) pairs.
/// Output before the first marker belongs to segment 0. Marker lines are
/// removed; surrounding blank lines are trimmed per segment.
///
/// A line is one of two things - a boundary or text - so the split is driven by
/// that classification rather than by a boolean match with an empty arm. The
/// index advances on every boundary, whether or not the part it closed carried
/// text, because the index is the segment's chain position and not its rank
/// among the parts that survived.
pub fn Fn(Output:&str) -> Vec<(usize, String)> {
	let mut Parts:Vec<(usize, String)> = Vec::new();
	let mut Current = String::new();
	let mut CurrentIndex = 0usize;

	Output.lines().for_each(|Line| {
		let Boundary = Line.trim() == SegMarker;

		let _ = Boundary.then(|| {
			let Text = Current.trim().to_string();

			let _ = (!Text.is_empty()).then(|| Parts.push((CurrentIndex, Text)));

			CurrentIndex += 1;
			// `clear()` rather than `Current = String::new()`: same observable
			// behaviour, but the buffer is reused - clippy's
			// `new_instead_of_clear` denies the reallocation under
			// `-D warnings` on the CI toolchain.
			Current.clear();
		});
		let _ = (!Boundary).then(|| {
			Current.push_str(Line);
			Current.push('\n');
		});
	});

	let Tail = Current.trim().to_string();
	let _ = (!Tail.is_empty()).then(|| Parts.push((CurrentIndex, Tail)));

	Parts
}
